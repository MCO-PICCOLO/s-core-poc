// Safe Exit Assist (SEA) — v1.0
//
// Activation modes:
//   Signal mode (default): waits for SIGRTMIN+2 from Timpani-N each tick.
//   Timer mode  (--timer [--period-ms N]): self-timed via sleep (standalone).
//
// Display modes:
//   TUI mode (default):     ratatui dashboard.  Press 'q' to quit, ↑↓/jk to scroll log.
//   Plain text (--no-tui):  line-by-line output, suitable for logging / CI.

use std::{
    collections::VecDeque,
    ffi::CString,
    io,
    time::{Duration, Instant, SystemTime},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        qos::{DataReaderQos, QosKind},
        qos_policy::{
            DurabilityQosPolicy, DurabilityQosPolicyKind, HistoryQosPolicy,
            HistoryQosPolicyKind, ReliabilityQosPolicy, ReliabilityQosPolicyKind,
        },
        status::NO_STATUS,
        time::{Duration as DdsDuration, DurationKind},
    },
    subscription::sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
};
use dust_dds_derive::DdsType;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};

// ── DDS message types (must match mini-adas publisher exactly) ────────────────

#[derive(Debug, Default, Clone, Serialize, Deserialize, DdsType)]
pub struct VehicleState {
    pub speed: f32,
    /// 0 = Park (P), 1 = Drive (D)
    pub gear: u8,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, DdsType)]
pub struct RearCameraScan {
    pub obstacle_detected: bool,
    pub distance: f32,
}

// ── Configuration ─────────────────────────────────────────────────────────────

struct Config {
    use_timer: bool,
    period_ms: u64,
    no_tui: bool,
}

impl Config {
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut cfg = Config { use_timer: false, period_ms: 100, no_tui: false };
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--timer"  => cfg.use_timer = true,
                "--no-tui" => cfg.no_tui = true,
                "--period-ms" | "-p" => {
                    i += 1;
                    if let Some(v) = args.get(i) {
                        cfg.period_ms = v.parse().unwrap_or(100);
                    }
                    cfg.use_timer = true;
                }
                _ => {}
            }
            i += 1;
        }
        cfg
    }

    fn mode_label(&self) -> String {
        let tick = if self.use_timer {
            format!("Timer ({}ms period)", self.period_ms)
        } else {
            format!("Timpani-N  SIGRTMIN+2 = {}", libc::SIGRTMIN() + 2)
        };
        let disp = if self.no_tui { "plain-text" } else { "TUI" };
        format!("{tick}   |   Display: {disp}")
    }
}

// ── Safety constants ──────────────────────────────────────────────────────────

const HAZARD_DIST_M: f32     = 7.0;
const CRITICAL_DIST_M: f32   = 3.0;
const DEADLINE_TARGET_MS: f64 = 10.0;

// ── Trend ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Trend { Unknown, Approaching, Stable, Receding, Critical }

impl Trend {
    fn label(self) -> &'static str {
        match self {
            Trend::Unknown     => "Unknown",
            Trend::Approaching => "Approaching",
            Trend::Stable      => "Stable",
            Trend::Receding    => "Receding",
            Trend::Critical    => "CRITICAL",
        }
    }
    fn color(self) -> Color {
        match self {
            Trend::Critical    => Color::Red,
            Trend::Approaching => Color::Yellow,
            Trend::Receding    => Color::Green,
            Trend::Stable      => Color::Cyan,
            Trend::Unknown     => Color::DarkGray,
        }
    }
}

// ── Drive mode ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum DriveMode { Moving, StoppedNotParked, Parked }

// ── App state ─────────────────────────────────────────────────────────────────

struct AppState {
    speed: f32,
    gear: u8,
    obstacle_detected: bool,
    distance: f32,
    prev_distance: Option<f32>,
    prev_distance_time: Instant,
    approach_velocity_ms: f32,
    trend: Trend,
    mode: DriveMode,
    dds_vs_received: bool,
    dds_rc_received: bool,
}

impl AppState {
    fn new() -> Self {
        Self {
            speed: 0.0,
            gear: 1,
            obstacle_detected: false,
            distance: 99.0,
            prev_distance: None,
            prev_distance_time: Instant::now(),
            approach_velocity_ms: 0.0,
            trend: Trend::Unknown,
            mode: DriveMode::Moving,
            dds_vs_received: false,
            dds_rc_received: false,
        }
    }

    fn update_obstacle(&mut self, detected: bool, dist: f32) {
        if !detected {
            self.obstacle_detected = false;
            self.distance = dist;
            self.prev_distance = None;
            self.approach_velocity_ms = 0.0;
            self.trend = Trend::Unknown;
            return;
        }
        if !self.obstacle_detected {
            self.obstacle_detected = true;
            self.distance = dist;
            self.prev_distance = Some(dist);
            self.prev_distance_time = Instant::now();
            self.approach_velocity_ms = 0.0;
            self.trend = Trend::Approaching;
            return;
        }
        let now = Instant::now();
        if let Some(prev) = self.prev_distance {
            let dt = now.duration_since(self.prev_distance_time).as_secs_f32();
            if dt > 0.005 {
                self.approach_velocity_ms = (prev - dist) / dt;
            }
            self.trend = if dist < CRITICAL_DIST_M && self.approach_velocity_ms > 0.05 {
                Trend::Critical
            } else if self.approach_velocity_ms > 0.1 {
                Trend::Approaching
            } else if self.approach_velocity_ms < -0.1 {
                Trend::Receding
            } else {
                Trend::Stable
            };
        }
        self.prev_distance = Some(dist);
        self.prev_distance_time = now;
        self.distance = dist;
        self.obstacle_detected = true;
    }

    fn update_mode(&mut self) {
        self.mode = if self.speed >= 0.5 {
            DriveMode::Moving
        } else if self.gear != 0 {
            DriveMode::StoppedNotParked
        } else {
            DriveMode::Parked
        };
    }

    fn is_hazard(&self) -> bool {
        self.mode == DriveMode::Parked
            && self.obstacle_detected
            && self.distance < HAZARD_DIST_M
    }

    fn gear_label(&self) -> &'static str {
        match self.gear { 0 => "Park (P)", 1 => "Drive (D)", _ => "Unknown" }
    }
}

// ── Event log (50-entry ring buffer with scroll) ──────────────────────────────

struct EventLog {
    entries: VecDeque<String>,
    /// 0 = following tail (newest visible); higher = scrolled back into history
    scroll_offset: usize,
}

impl EventLog {
    fn new() -> Self {
        Self { entries: VecDeque::with_capacity(50), scroll_offset: 0 }
    }

    fn push(&mut self, msg: impl Into<String>) {
        if self.entries.len() >= 50 { self.entries.pop_front(); }
        self.entries.push_back(format!("[{}]  {}", wall_time(), msg.into()));
    }

    fn scroll_up(&mut self, visible: usize) {
        let max = self.entries.len().saturating_sub(visible);
        self.scroll_offset = (self.scroll_offset + 1).min(max);
    }

    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn jump_to_end(&mut self) {
        self.scroll_offset = 0;
    }
}

// ── Signal handling (Timpani-N integration) ───────────────────────────────────

fn set_process_name(name: &str) {
    if let Ok(c) = CString::new(name) {
        unsafe { libc::prctl(libc::PR_SET_NAME, c.as_ptr() as libc::c_ulong, 0, 0, 0); }
    }
}

fn block_tt_signal() -> libc::sigset_t {
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        libc::sigaddset(&mut set, libc::SIGRTMIN() + 2);
        libc::sigprocmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
        set
    }
}

fn wait_for_tt_signal(set: &libc::sigset_t) {
    unsafe {
        let mut received = 0i32;
        while libc::sigwait(set, &mut received) != 0 {}
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn wall_time() -> String {
    let s = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default().as_secs();
    format!("{:02}:{:02}:{:02}", (s / 3600) % 24, (s / 60) % 60, s % 60)
}

fn make_reader_qos() -> DataReaderQos {
    DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(DdsDuration::new(0, 100_000_000)),
        },
        durability: DurabilityQosPolicy { kind: DurabilityQosPolicyKind::TransientLocal },
        history: HistoryQosPolicy { kind: HistoryQosPolicyKind::KeepLast(5) },
        ..Default::default()
    }
}

// ── TUI panel drawers ─────────────────────────────────────────────────────────

fn draw_title(f: &mut Frame, area: Rect, mode_label: &str) {
    let p = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  SAFE EXIT ASSIST (SEA) \u{2014} v1.0",
                Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("    |    Process: sea_app    |    DDS Domain: 100"),
        ]),
        Line::from(vec![
            Span::styled("  Mode: ", Style::default().fg(Color::DarkGray)),
            Span::styled(mode_label, Style::default().fg(Color::Cyan)),
            Span::styled("    \u{2502}    [q] quit    [\u{2191}\u{2193}/jk] scroll log    [G/End] tail",
                Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_vehicle_state(f: &mut Frame, area: Rect, state: &AppState) {
    let (mode_str, mode_col) = match state.mode {
        DriveMode::Moving           => ("MOVING",          Color::Yellow),
        DriveMode::StoppedNotParked => ("STOPPED (not P)", Color::Cyan),
        DriveMode::Parked           => ("PARKED",          Color::Green),
    };
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Speed  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.1} km/h", state.speed),
                Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Gear   : ", Style::default().fg(Color::DarkGray)),
            Span::raw(state.gear_label()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Mode   : ", Style::default().fg(Color::DarkGray)),
            Span::styled(mode_str,
                Style::default().fg(mode_col).add_modifier(Modifier::BOLD)),
        ]),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Vehicle State ")),
        area,
    );
}

fn draw_rear_sensor(f: &mut Frame, area: Rect, state: &AppState) {
    let (obs_str, obs_col) = if state.obstacle_detected {
        ("YES  \u{26A0}", Color::Red)
    } else {
        ("No   \u{2713}", Color::Green)
    };

    // Distance / velocity / trend are only meaningful when an obstacle is present
    let (dist_span, vel_span, trend_span) = if state.obstacle_detected {
        let dist_col = if state.distance < HAZARD_DIST_M { Color::Red } else { Color::Green };
        (
            Span::styled(format!("{:.2} m", state.distance),
                Style::default().fg(dist_col).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:+.2} m/s", state.approach_velocity_ms)),
            Span::styled(state.trend.label(),
                Style::default().fg(state.trend.color())),
        )
    } else {
        let dim = Style::default().fg(Color::DarkGray);
        (
            Span::styled("---", dim),
            Span::styled("---", dim),
            Span::styled("---", dim),
        )
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Obstacle : ", Style::default().fg(Color::DarkGray)),
            Span::styled(obs_str,
                Style::default().fg(obs_col).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Distance : ", Style::default().fg(Color::DarkGray)),
            dist_span,
        ]),
        Line::from(vec![
            Span::styled("  Velocity : ", Style::default().fg(Color::DarkGray)),
            vel_span,
        ]),
        Line::from(vec![
            Span::styled("  Trend    : ", Style::default().fg(Color::DarkGray)),
            trend_span,
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Rear Sensor ")),
        area,
    );
}

fn draw_decision(f: &mut Frame, area: Rect, state: &AppState) {
    let (msg, bg, fg) = if state.is_hazard() {
        (
            format!("  \u{1F512}  DOOR_LOCK_ENGAGED \u{2014} HAZARD: Object at {:.2}m  (threshold {:.0}m)",
                state.distance, HAZARD_DIST_M),
            Color::Red, Color::White,
        )
    } else {
        match state.mode {
            DriveMode::Moving => (
                format!("  \u{1F697}  SEA INHIBITED \u{2014} Vehicle moving at {:.1} km/h", state.speed),
                Color::Yellow, Color::Black,
            ),
            DriveMode::StoppedNotParked => (
                "  \u{23F3}  SEA WAITING \u{2014} Vehicle stopped \u{2014} shift to Park to enable".to_string(),
                Color::DarkGray, Color::White,
            ),
            DriveMode::Parked => (
                format!("  \u{2705}  SAFE TO OPEN \u{2014} Parked, nearest object at {:.2}m  (threshold {:.0}m)",
                    state.distance, HAZARD_DIST_M),
                Color::Green, Color::Black,
            ),
        }
    };
    let p = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(msg,
            Style::default().fg(fg).add_modifier(Modifier::BOLD))),
    ])
    .style(Style::default().bg(bg))
    .block(Block::default().borders(Borders::ALL).title(" Safety Decision ")
        .style(Style::default().bg(bg).fg(fg)));
    f.render_widget(p, area);
}

fn draw_performance(f: &mut Frame, area: Rect, latency_ms: f64) {
    let (lat_col, status) = if latency_ms > DEADLINE_TARGET_MS {
        (Color::Red,   "MISS \u{26A0}")
    } else {
        (Color::Green, "OK   \u{2713}")
    };
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Latency : ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:.2} ms", latency_ms),
                Style::default().fg(lat_col).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Target  : ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:.0} ms", DEADLINE_TARGET_MS)),
        ]),
        Line::from(vec![
            Span::styled("  Status  : ", Style::default().fg(Color::DarkGray)),
            Span::styled(status, Style::default().fg(lat_col)),
        ]),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Performance ")),
        area,
    );
}

fn draw_event_log(f: &mut Frame, area: Rect, log: &EventLog) {
    // Usable rows inside the border
    let visible = (area.height as usize).saturating_sub(2);
    let total   = log.entries.len();

    // scroll_offset 0 = tail (newest); higher = older history
    // The window starts at: total - visible - scroll_offset  (clamped to 0)
    let start = total
        .saturating_sub(visible)
        .saturating_sub(log.scroll_offset);

    let items: Vec<ListItem> = log.entries.iter()
        .skip(start)
        .take(visible)
        .enumerate()
        .map(|(_, e)| ListItem::new(format!("  {e}")))
        .collect();

    let title = if log.scroll_offset > 0 {
        // Show user they are scrolled back; pressing G/End returns to tail
        format!(" Event Log  [{}/{}]  \u{2191}\u{2193} k/j   G/End = tail ",
            start + 1, total)
    } else {
        format!(" Event Log  [{} entries]  \u{2191}\u{2193} k/j  G/End = tail ", total)
    };

    f.render_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn render_frame(f: &mut Frame, state: &AppState, log: &EventLog,
                latency_ms: f64, mode_label: &str) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // title + mode line
            Constraint::Length(9),  // vehicle state | rear sensor
            Constraint::Length(5),  // safety decision
            Constraint::Min(0),     // performance | event log
        ])
        .split(f.area());

    draw_title(f, rows[0], mode_label);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);
    draw_vehicle_state(f, mid[0], state);
    draw_rear_sensor(f, mid[1], state);

    draw_decision(f, rows[2], state);

    let bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(rows[3]);
    draw_performance(f, bot[0], latency_ms);
    draw_event_log(f, bot[1], log);
}

// ── Plain text fallback (--no-tui) ────────────────────────────────────────────

fn render_plain(state: &AppState, latency_ms: f64) {
    use io::Write;
    match state.mode {
        DriveMode::Moving => {
            print!("\r[IDLE ]  Speed: {:5.1} km/h  Gear: {:10}  SEA: Inhibited              ",
                state.speed, state.gear_label());
            let _ = io::stdout().flush();
            return;
        }
        DriveMode::StoppedNotParked =>
            println!("[WAIT ]  Stopped \u{2014} gear: {}  (shift to Park to activate)", state.gear_label()),
        DriveMode::Parked => {
            if state.obstacle_detected {
                let haz = if state.is_hazard() { "  *** HAZARD \u{2014} DOOR LOCKED ***" } else { "" };
                println!("[SCAN ]  Dist: {:.2}m  Vel: {:+.2} m/s  Trend: {:12}{haz}",
                    state.distance, state.approach_velocity_ms, state.trend.label());
            } else {
                println!("[SCAN ]  Rear: Clear \u{2014} SAFE TO OPEN");
            }
        }
    }
    if latency_ms > DEADLINE_TARGET_MS {
        println!("[PERF ]  Latency: {:.2}ms  WARNING: target {:.0}ms missed!",
                 latency_ms, DEADLINE_TARGET_MS);
    } else {
        println!("[PERF ]  Latency: {:.2}ms  target {:.0}ms  OK", latency_ms, DEADLINE_TARGET_MS);
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let config = Config::from_args();

    // Register process name BEFORE DDS creates threads (sig mask is inherited)
    set_process_name("sea_app");
    let sig_set = block_tt_signal();
    let mode_label = config.mode_label();

    // ── DDS setup
    let participant = DomainParticipantFactory::get_instance()
        .create_participant(100, QosKind::Default, None, NO_STATUS)
        .expect("[DDS] FATAL: create_participant on domain 100 failed");

    let subscriber = participant
        .create_subscriber(QosKind::Default, None, NO_STATUS)
        .expect("create_subscriber failed");

    let vs_topic = participant
        .create_topic::<VehicleState>(
            "vehicle/state", "VehicleState", QosKind::Default, None, NO_STATUS)
        .expect("create vehicle/state topic failed");
    let vs_reader = subscriber
        .create_datareader::<VehicleState>(
            &vs_topic, QosKind::Specific(make_reader_qos()), None, NO_STATUS)
        .expect("create vs_reader failed");

    let rc_topic = participant
        .create_topic::<RearCameraScan>(
            "sensor/rear_camera", "RearCameraScan", QosKind::Default, None, NO_STATUS)
        .expect("create sensor/rear_camera topic failed");
    let rc_reader = subscriber
        .create_datareader::<RearCameraScan>(
            &rc_topic, QosKind::Specific(make_reader_qos()), None, NO_STATUS)
        .expect("create rc_reader failed");

    let mut state = AppState::new();
    let mut log   = EventLog::new();
    log.push("SEA module started \u{2014} DDS domain 100");

    // Closure: poll both DDS readers, update state and event log
    let poll_dds = |st: &mut AppState, lg: &mut EventLog| {
        if let Ok(samples) = vs_reader.take(32, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE) {
            if let Some(s) = samples.last() {
                if let Ok(data) = s.data() {
                    let prev_gear = st.gear;
                    st.speed = data.speed;
                    st.gear  = data.gear;
                    if !st.dds_vs_received {
                        lg.push("DDS: vehicle/state connected");
                        st.dds_vs_received = true;
                    }
                    if data.gear != prev_gear {
                        if data.gear == 0 {
                            lg.push("Gear -> Park \u{2014} SEA activating");
                        } else {
                            lg.push("Gear -> Drive \u{2014} SEA inhibited");
                        }
                    }
                }
            }
        }
        if let Ok(samples) = rc_reader.take(32, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE) {
            if let Some(s) = samples.last() {
                if let Ok(data) = s.data() {
                    let prev_detected = st.obstacle_detected;
                    let prev_hazard   = st.is_hazard();
                    st.update_obstacle(data.obstacle_detected, data.distance);
                    if !st.dds_rc_received {
                        lg.push("DDS: sensor/rear_camera connected");
                        st.dds_rc_received = true;
                    }
                    if data.obstacle_detected && !prev_detected {
                        lg.push(format!("Obstacle detected at {:.2}m", data.distance));
                    } else if !data.obstacle_detected && prev_detected {
                        lg.push("Obstacle cleared \u{2014} path free");
                    }
                    let new_hazard = st.is_hazard();
                    if new_hazard && !prev_hazard {
                        lg.push(format!("HAZARD: {:.2}m < {:.0}m \u{2014} DOOR LOCKED",
                                        st.distance, HAZARD_DIST_M));
                    } else if !new_hazard && prev_hazard {
                        lg.push("Hazard cleared \u{2014} door may open");
                    }
                }
            }
        }
    };

    // ── Plain text mode ───────────────────────────────────────────────────────
    if config.no_tui {
        println!("[INIT]   Safe Exit Assist v1.0");
        println!("[MODE]   {}", mode_label);
        println!("[DDS ]   Subscribed: vehicle/state  +  sensor/rear_camera");
        if !config.use_timer {
            println!("[STATUS] Waiting for Timpani-N activation (SIGRTMIN+2 = {})...",
                     libc::SIGRTMIN() + 2);
        }
        println!();
        loop {
            if config.use_timer {
                std::thread::sleep(Duration::from_millis(config.period_ms));
            } else {
                wait_for_tt_signal(&sig_set);
            }
            let t0 = Instant::now();
            poll_dds(&mut state, &mut log);
            state.update_mode();
            let latency = t0.elapsed().as_secs_f64() * 1000.0;
            render_plain(&state, latency);
        }
    }

    // ── TUI mode ──────────────────────────────────────────────────────────────
    else {
        enable_raw_mode().expect("enable_raw_mode");
        execute!(io::stdout(), EnterAlternateScreen).expect("EnterAlternateScreen");
        let backend      = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend).expect("Terminal::new");
        let mut latency_ms = 0.0f64;

        let result: io::Result<()> = (|| {
            loop {
                // Approximate visible rows in the log panel for scroll boundary calculation.
                // Layout: title(4) + mid(9) + decision(5) + 2 border lines = 20 overhead.
                let log_visible = (terminal.size()?.height as usize).saturating_sub(20);

                // Handle a single key event; returns true if the app should quit.
                let handle_key = |k: crossterm::event::KeyEvent,
                                  lg: &mut EventLog,
                                  vis: usize| -> bool {
                    if k.kind != KeyEventKind::Press { return false; }
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => true,
                        KeyCode::Up    | KeyCode::Char('k') => { lg.scroll_up(vis);   false }
                        KeyCode::Down  | KeyCode::Char('j') => { lg.scroll_down();    false }
                        KeyCode::End   | KeyCode::Char('G') => { lg.jump_to_end();    false }
                        _ => false,
                    }
                };

                if config.use_timer {
                    // Sleep in 20 ms slices so key input is snappy
                    let deadline = Instant::now() + Duration::from_millis(config.period_ms);
                    loop {
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        if remaining.is_zero() { break; }
                        if event::poll(remaining.min(Duration::from_millis(20)))? {
                            if let Event::Key(k) = event::read()? {
                                if handle_key(k, &mut log, log_visible) {
                                    return Ok(());
                                }
                            }
                        }
                    }
                } else {
                    // Signal mode: block on Timpani-N tick, then drain all pending keys
                    wait_for_tt_signal(&sig_set);
                    while event::poll(Duration::ZERO)? {
                        if let Event::Key(k) = event::read()? {
                            if handle_key(k, &mut log, log_visible) {
                                return Ok(());
                            }
                        }
                    }
                }

                // Poll DDS + update state
                let t0 = Instant::now();
                poll_dds(&mut state, &mut log);
                state.update_mode();
                latency_ms = t0.elapsed().as_secs_f64() * 1000.0;

                // Redraw TUI
                terminal.draw(|f| {
                    render_frame(f, &state, &log, latency_ms, &mode_label)
                })?;
            }
        })();

        // Always restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
        let _ = terminal.show_cursor();

        if let Err(e) = result {
            eprintln!("TUI error: {e}");
            std::process::exit(1);
        }
    }
}
