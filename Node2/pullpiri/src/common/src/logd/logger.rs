//! Async logging subsystem: callers enqueue `LogEnvelope`s into bounded
//! queues keyed by virtual channels, while a background worker drains the
//! queues and forwards payloads via Unix datagram sockets.

use bytes::BytesMut;
use prost::Message;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::net::UnixDatagram;
use tokio::runtime::Handle;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

use crate::logd::LogEnvelope;

/// Global singleton that holds the active async logger instance.
static LOGGER: OnceLock<AsyncLogger> = OnceLock::new();

/// Bounded FIFO queue that drops the oldest entry when capacity is reached.
struct BoundedQueue<LogEnvelope> {
    inner: Mutex<VecDeque<LogEnvelope>>,
    capacity: usize,
}

impl BoundedQueue<LogEnvelope> {
    /// Construct a queue with the given capacity.
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of items to retain.
    fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    /// Push an item, dropping the oldest element if the queue is full.
    ///
    /// # Arguments
    /// * `item` - Envelope to enqueue.
    async fn push_drop_oldest(&self, item: LogEnvelope) {
        let mut guard = self.inner.lock().await;
        if guard.len() == self.capacity {
            guard.pop_front();
        }
        guard.push_back(item);
    }

    /// Drain all pending items, returning them as a `Vec`.
    ///
    /// # Returns
    /// All enqueued envelopes in FIFO order.
    async fn drain(&self) -> Vec<LogEnvelope> {
        let mut guard = self.inner.lock().await;
        guard.drain(..).collect()
    }

    /// Reinsert a batch of items at the front so they are retried first.
    ///
    /// # Arguments
    /// * `items` - Envelopes to re-queue.
    async fn push_front_batch(&self, mut items: Vec<LogEnvelope>) {
        if items.is_empty() {
            return;
        }

        let mut guard = self.inner.lock().await;
        while let Some(item) = items.pop() {
            guard.push_front(item);
            if guard.len() > self.capacity {
                guard.pop_back();
            }
        }
    }
}

/// Logical channels supported by the logger.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Ch {
    Logd,
}

impl Ch {
    /// Return the Unix socket path for the given channel.
    fn socket_path(self) -> &'static str {
        match self {
            Ch::Logd => crate::logd::LOGD_SOCKET_PATH,
        }
    }
}

/// Result state returned by `drain_channel`.
enum DrainState {
    Idle,
    Pending,
}

/// Aggregated state for the global async logger instance.
pub struct AsyncLogger {
    q: HashMap<Ch, Arc<BoundedQueue<LogEnvelope>>>,
    notify_tx: Sender<()>,
    tag: String,
}

/// Initialize the async logger for the given tag and spawn the worker task.
///
/// # Arguments
/// * `tag` - Tag field to stamp on outgoing envelopes.
///
/// # Errors
/// Propagates I/O errors from socket creation or queue setup.
pub async fn init_async_logger(tag: &str) -> std::io::Result<()> {
    let logd_q = Arc::new(BoundedQueue::<LogEnvelope>::new(8192));
    let (tx, rx) = channel::<()>(1);

    let mut q = HashMap::new();
    q.insert(Ch::Logd, logd_q.clone());

    let logger = AsyncLogger {
        q,
        notify_tx: tx.clone(),
        tag: tag.to_string(),
    };
    let _ = LOGGER.set(logger);

    spawn_worker(rx, logd_q).await;

    Ok(())
}

/// Convenience API for async contexts: await enqueue completion and log
/// failures to stderr.
///
/// # Arguments
/// * `level` - Severity level code.
/// * `message` - Formatted log message.
pub async fn log(level: i32, message: String) {
    if let Err(err) = enqueue(level, message).await {
        crate::logd!(6, "logger enqueue failed: {err}");
    }
}

/// Fire-and-forget API for synchronous call sites. Spawns a task on the
/// current Tokio runtime (if any) to enqueue the log message.
///
/// # Arguments
/// * `level` - Severity level code.
/// * `message` - Formatted log message.
pub fn log_nowait(level: i32, message: String) {
    match Handle::try_current() {
        Ok(handle) => {
            handle.spawn(async move {
                if let Err(err) = enqueue(level, message).await {
                    crate::logd!(6, "logger enqueue failed: {err}");
                }
            });
        }
        Err(_) => {
            crate::logd!(4, "logger not running inside a Tokio runtime; dropping log");
        }
    }
}

/// Core enqueue function shared by `log` and `log_nowait`.
///
/// # Arguments
/// * `level` - Severity level code.
/// * `message` - Formatted log message.
///
/// # Errors
/// Returns an error when the logger is not initialized or the notify
/// channel has been closed.
pub async fn enqueue(level: i32, message: String) -> std::io::Result<()> {
    let Some(gl) = LOGGER.get() else {
        return Err(std::io::Error::other("logger not initialized"));
    };

    let env = LogEnvelope {
        ts_real_ns: real_time_ns(),
        tag: gl.tag.clone(),
        level,
        message,
    };

    let q = gl.q.get(&Ch::Logd).unwrap();
    q.push_drop_oldest(env).await;

    match gl.notify_tx.try_send(()) {
        Ok(()) | Err(TrySendError::Full(_)) => Ok(()),
        Err(TrySendError::Closed(_)) => Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "logger worker not running",
        )),
    }
}

/// Spawn the background worker that drains the queue whenever an enqueue
/// notification is received.
///
/// # Arguments
/// * `notify_rx` - Receiver for edge-triggered wakeups.
/// * `logd_q` - Queue storing outgoing envelopes.
async fn spawn_worker(mut notify_rx: Receiver<()>, logd_q: Arc<BoundedQueue<LogEnvelope>>) {
    tokio::spawn(async move {
        let mut socks: HashMap<Ch, (UnixDatagram, bool)> = HashMap::new();

        let sock = UnixDatagram::unbound().expect("unbound sock");
        socks.insert(Ch::Logd, (sock, false));

        while notify_rx.recv().await.is_some() {
            loop {
                match drain_channel(Ch::Logd, &logd_q, &mut socks).await {
                    DrainState::Idle => break,
                    DrainState::Pending => continue,
                }
            }
        }

        while matches!(
            drain_channel(Ch::Logd, &logd_q, &mut socks).await,
            DrainState::Pending
        ) {}
    });
}

/// Drain a single channel queue and forward its messages to the connected
/// Unix datagram socket.
///
/// # Arguments
/// * `ch` - Logical channel identifier.
/// * `q` - Queue backing the channel.
/// * `socks` - Cached sockets paired with connection status flags.
///
/// # Returns
/// `DrainState::Idle` when no work remains, otherwise `DrainState::Pending`.
async fn drain_channel(
    ch: Ch,
    q: &BoundedQueue<LogEnvelope>,
    socks: &mut HashMap<Ch, (UnixDatagram, bool)>,
) -> DrainState {
    let (sock, connected) = socks.get_mut(&ch).unwrap();
    let batch = q.drain().await;

    if batch.is_empty() {
        return DrainState::Idle;
    }

    if !*connected {
        if sock.connect(ch.socket_path()).is_ok() {
            *connected = true;
        } else {
            q.push_front_batch(batch).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
            return DrainState::Pending;
        }
    }

    let mut iter = batch.into_iter();
    while let Some(env) = iter.next() {
        print_stdout(&env);
        let mut buf = BytesMut::with_capacity(env.encoded_len());
        if env.encode(&mut buf).is_err() {
            continue;
        }
        if sock.send(&buf).await.is_err() {
            *connected = false;
            let mut retry_items = vec![env];
            retry_items.extend(iter);
            q.push_front_batch(retry_items).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
            return DrainState::Pending;
        }
    }

    DrainState::Idle
}

fn print_stdout(env: &LogEnvelope) {
    use chrono::{DateTime, Local};
    use std::time::{Duration, UNIX_EPOCH};

    let sys_time = UNIX_EPOCH + Duration::from_nanos(env.ts_real_ns);
    let chrono_time: DateTime<Local> = DateTime::from(sys_time);
    let time_str = chrono_time.format("%Y-%m-%d %H:%M:%S%.3f");
    let tag = env.tag.clone();
    let message = env.message.clone();

    let level = match env.level {
        1 => "V",
        2 => "D",
        3 => "I",
        4 => "W",
        5 => "E",
        6 => "F",
        _ => "?",
    };

    println!(
        "{:<24} │ {:<2} │ {:<30} │ {}",
        time_str, level, tag, message
    );
}

/// Read the current realtime clock as an absolute nanosecond value.
fn real_time_ns() -> u64 {
    unsafe {
        let mut ts: libc::timespec = std::mem::zeroed();
        libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
        (ts.tv_sec as u64) * 1_000_000_000u64 + (ts.tv_nsec as u64)
    }
}
