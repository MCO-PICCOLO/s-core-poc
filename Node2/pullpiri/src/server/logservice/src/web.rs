// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Minimal Axum-based HTTP server that streams log lines to browsers via SSE.

use std::collections::VecDeque;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        Html,
    },
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};

use crate::log_entry::LogEvent;

/// Shared state for the HTTP server, mainly the broadcast sender of log lines.
#[derive(Clone)]
pub struct WebState {
    pub log_tx: broadcast::Sender<LogEvent>,
    pub log_history: Arc<Mutex<VecDeque<LogEvent>>>,
}

/// Default address (`0.0.0.0:47097`) for the built-in log viewer.
pub fn default_http_addr() -> SocketAddr {
    //SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 47097)
    SocketAddr::new(
        common::setting::get_config().host.ip.parse().unwrap(),
        47097,
    )
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>PULLPIRI LOG STREAM</title>
    <style>
      :root {
        color-scheme: dark;
        --bg: #050914;
        --panel: #0f172a;
        --accent: #34d399;
        --border: #1f2a44;
        --muted: #7dd3fc;
      }
      * { box-sizing: border-box; }
      body { font-family: "JetBrains Mono", Consolas, monospace; background: var(--bg); color: #e2e8f0; margin: 0; height: 100vh; overflow: hidden; }
      main { height: 100vh; display: flex; flex-direction: column; }
      .hero { flex: 0 0 25vh; padding: 2.5rem 3rem; display: flex; flex-direction: column; justify-content: flex-end; }
      .eyebrow { text-transform: uppercase; letter-spacing: 0.2em; color: var(--muted); font-size: 0.75rem; margin-bottom: 0.75rem; }
      h1 { font-size: 2.2rem; margin: 0; letter-spacing: 0.08em; }
      .subtitle { margin-top: 0.75rem; color: #94a3b8; }
      .toggle-grid { margin-top: 1.5rem; display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 0.5rem; }
      .toggle-btn { border: 1px solid var(--border); background: rgba(15, 23, 42, 0.65); color: #e2e8f0; padding: 0.65rem 0.9rem; border-radius: 999px; font-family: inherit; font-size: 0.8rem; letter-spacing: 0.06em; text-transform: uppercase; cursor: pointer; transition: border-color 0.2s ease, background 0.2s ease, color 0.2s ease; }
      .toggle-btn.is-active { border-color: var(--accent); background: rgba(52, 211, 153, 0.15); color: var(--accent); box-shadow: 0 0 0 1px rgba(52, 211, 153, 0.25); }
      .log-panel { flex: 1; background: var(--panel); border-radius: 28px 28px 0 0; padding: 1.5rem 2rem 2rem; box-shadow: inset 0 1px 0 var(--border); display: flex; flex-direction: column; gap: 0.75rem; overflow: hidden; }
      .log-header, .log-row { display: grid; grid-template-columns: 210px 60px 220px 1fr; gap: 0.75rem; align-items: center; padding: 0.35rem 0.25rem; }
      .log-header { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.2em; color: #94a3b8; border-bottom: 1px solid var(--border); }
      .log-rows { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 0.15rem; }
      .log-row { font-size: 0.9rem; border-bottom: 1px dashed #17203a; padding-bottom: 0.35rem; }
      .log-cell { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding-right: 0.5rem; border-right: 1px solid var(--border); }
      .log-cell:last-child { border-right: none; white-space: normal; }
      .message-cell { white-space: normal; word-break: break-word; }
      .level-badge { font-weight: bold; color: #cbd5e1; }
      .log-rows::-webkit-scrollbar { width: 8px; }
      .log-rows::-webkit-scrollbar-thumb { background: var(--border); border-radius: 4px; }
    </style>
  </head>
  <body>
    <main>
      <section class="hero">
        <p class="eyebrow">Realtime Monitor</p>
        <h1>PULLPIRI LOG STREAM</h1>
        <p class="subtitle">Live feed sourced from the Unix domain socket.</p>
        <div class="toggle-grid">
          <button class="toggle-btn is-active" type="button" data-tag="actioncontroller">action controller</button>
          <button class="toggle-btn is-active" type="button" data-tag="filtergateway">filter gateway</button>
          <button class="toggle-btn is-active" type="button" data-tag="statemanager">state manager</button>
          <button class="toggle-btn is-active" type="button" data-tag="apiserver">api server</button>
          <button class="toggle-btn is-active" type="button" data-tag="monitoringserver">monitoring server</button>
          <button class="toggle-btn is-active" type="button" data-tag="policymanager">policy manager</button>
          <button class="toggle-btn is-active" type="button" data-tag="settingsservice">settingsservice</button>
          <button class="toggle-btn is-active" type="button" data-tag="resourcemanager">resource manager</button>
        </div>
      </section>
      <section class="log-panel">
        <div class="log-header">
          <span>Timestamp</span>
          <span>Lv</span>
          <span>Tag</span>
          <span>Message</span>
        </div>
        <div id="log-rows" class="log-rows"></div>
      </section>
    </main>
    <script>
      const rows = document.getElementById('log-rows');
      const MAX_ROWS = 2000;
      const SCROLL_MARGIN = 48;
      let autoScroll = true;
      const logBuffer = [];
      const levelColors = {
        '?': '#cbd5e1',
        V: '#94a3b8',
        D: '#818cf8',
        I: '#6ee7b7',
        W: '#fcd34d',
        E: '#f87171',
        F: '#ef4444',
        default: '#cbd5e1',
      };
      const toggleButtons = Array.from(document.querySelectorAll('.toggle-btn'));
      const controllableTags = new Set(toggleButtons.map((btn) => btn.dataset.tag));
      const activeTags = new Set(controllableTags);
      const source = new EventSource('/logs');
      source.onmessage = (event) => {
        try {
          const payload = JSON.parse(event.data);
          handleLogEvent(payload);
        } catch (err) {
          console.error('Failed to parse log event', err);
        }
      };

      function handleLogEvent(entry) {
        logBuffer.push(entry);
        if (logBuffer.length > MAX_ROWS) {
          const removed = logBuffer.shift();
          if (shouldDisplay(removed.tag) && rows.firstChild) {
            rows.removeChild(rows.firstChild);
          }
        }

        if (shouldDisplay(entry.tag)) {
          rows.appendChild(buildRow(entry));
          trimDisplayedRows();
          if (autoScroll) {
            scrollToBottom();
          }
        }
      }

      function buildRow({ timestamp, level, tag, message }) {
        const row = document.createElement('div');
        row.className = 'log-row';

        row.appendChild(createCell(timestamp));
        const levelCell = createCell(level);
        levelCell.classList.add('level-badge');
        applyLevelColor(levelCell, level);
        row.appendChild(levelCell);
        row.appendChild(createCell(tag));
        const msgCell = createCell(message);
        msgCell.classList.add('message-cell');
        applyLevelColor(msgCell, level);
        row.appendChild(msgCell);

        return row;
      }

      function createCell(text) {
        const cell = document.createElement('span');
        cell.className = 'log-cell';
        cell.textContent = text;
        return cell;
      }

      function applyLevelColor(cell, level) {
        cell.style.color = levelColors[level] || levelColors.default;
      }

      function shouldDisplay(tag) {
        return !controllableTags.has(tag) || activeTags.has(tag);
      }

      function trimDisplayedRows() {
        while (rows.children.length > MAX_ROWS) {
          rows.removeChild(rows.firstChild);
        }
      }

      function scrollToBottom() {
        rows.scrollTop = rows.scrollHeight;
      }

      function getDistanceFromBottom() {
        return rows.scrollHeight - (rows.scrollTop + rows.clientHeight);
      }

      function renderFilteredRows() {
        const previousDistance = getDistanceFromBottom();
        rows.innerHTML = '';
        logBuffer.forEach((entry) => {
          if (shouldDisplay(entry.tag)) {
            rows.appendChild(buildRow(entry));
          }
        });
        trimDisplayedRows();
        restoreScroll(previousDistance);
      }

      function restoreScroll(previousDistance) {
        if (autoScroll) {
          scrollToBottom();
          return;
        }
        const target = rows.scrollHeight - rows.clientHeight - previousDistance;
        rows.scrollTop = Math.max(target, 0);
      }

      rows.addEventListener('scroll', () => {
        autoScroll = getDistanceFromBottom() <= SCROLL_MARGIN;
      });

      toggleButtons.forEach((btn) => {
        btn.addEventListener('click', () => {
          btn.classList.toggle('is-active');
          const tag = btn.dataset.tag;
          if (btn.classList.contains('is-active')) {
            activeTags.add(tag);
          } else {
            activeTags.delete(tag);
          }
          renderFilteredRows();
        });
      });
    </script>
  </body>
</html>
"#;

/// Launch the HTTP server and keep serving until the task is cancelled.
pub async fn run_http_server(state: WebState, addr: SocketAddr) {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/logs", get(stream_logs))
        .with_state(state);

    match TcpListener::bind(addr).await {
        Ok(listener) => {
            if let Err(err) = axum::serve(listener, app.into_make_service()).await {
                eprintln!("[aggregator] http server error: {err}");
            }
        }
        Err(err) => eprintln!("[aggregator] failed to bind http listener: {err}"),
    }
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn stream_logs(
    State(state): State<WebState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let history_events = {
        let history = state.log_history.lock().await;
        history
            .iter()
            .filter_map(|entry| match Event::default().json_data(entry) {
                Ok(event) => Some(Ok(event)),
                Err(err) => {
                    eprintln!("[aggregator] failed to encode log history for SSE: {err}");
                    None
                }
            })
            .collect::<Vec<_>>()
    };

    let history_stream = tokio_stream::iter(history_events);

    let live_stream = BroadcastStream::new(state.log_tx.subscribe()).filter_map(|msg| match msg {
        Ok(entry) => match Event::default().json_data(&entry) {
            Ok(event) => Some(Ok(event)),
            Err(err) => {
                eprintln!("[aggregator] failed to encode log for SSE: {err}");
                None
            }
        },
        Err(_) => None,
    });

    let stream = history_stream.chain(live_stream);

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
