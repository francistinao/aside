//! A session: the script being read, the timing engine and the thread that drives it.
//! The thread waits on a channel until the next boundary or a control message, then emits
//! only what changed. It never touches windows.

pub mod engine;
pub mod tokenize;

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use engine::{Clock, Engine, Snapshot, StateKind, SystemClock};
use tokenize::Token;

use crate::error::Result;
use crate::lock::lock;
use crate::prompter;

pub const SCRIPT_EVENT: &str = "session://script";
pub const POSITION_EVENT: &str = "session://position";
pub const STATE_EVENT: &str = "session://state";
/// Windows that follow the session.
const LISTENERS: [&str; 2] = [prompter::LABEL, prompter::LIBRARY];
/// With nothing scheduled the thread sleeps until a message arrives.
const IDLE_WAIT: Duration = Duration::from_secs(3600);

/// How the bubble highlights. Line mode emits positions only when the line changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Highlight {
    #[default]
    Word,
    Line,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Pause,
    Resume,
    Restart,
    Stop,
    Prev,
    Next,
}

/// Sent once per session to the prompter, and again when its page reloads.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptPayload {
    pub title: String,
    pub tokens: Vec<Token>,
    pub words: u32,
    pub total: f64,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PositionPayload {
    word: Option<u32>,
    line: u32,
    section: u32,
    elapsed: f64,
    total: f64,
    pause: Option<u32>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatePayload {
    state: StateKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u8>,
}

pub enum Control {
    Start(ScriptPayload),
    SetLines(Vec<(u32, u32)>),
    Act(Action),
    SetWpm(u32),
    SetHighlight(Highlight),
    /// Re-emit the script and the current state, for a prompter page that just loaded.
    Resend,
}

/// What the rest of the app may read about the session without asking the thread:
/// the hide toggle pauses a running script, and the menu bar extra shows the progress.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    pub state: StateKind,
    pub title: String,
    pub elapsed: f64,
    pub total: f64,
    pub highlight: Highlight,
}

/// Talks to the session thread. Managed by the Tauri builder.
pub struct SessionHandle {
    tx: Sender<Control>,
    status: Arc<Mutex<Status>>,
}

impl SessionHandle {
    pub fn send(&self, control: Control) {
        // A closed channel means the thread is gone with the app. Nothing to do.
        let _ = self.tx.send(control);
    }

    /// The last state the thread published. A control just sent may not show yet.
    pub fn status(&self) -> Status {
        lock(&self.status).clone()
    }
}

pub fn spawn(app: AppHandle, wpm: u32, highlight: Highlight) -> Result<SessionHandle> {
    let (tx, rx) = mpsc::channel();
    let status = Arc::new(Mutex::new(Status {
        highlight,
        ..Status::default()
    }));
    let shared = Arc::clone(&status);
    thread::Builder::new()
        .name("session".into())
        .spawn(move || run(app, rx, SystemClock, wpm, highlight, shared))?;
    Ok(SessionHandle { tx, status })
}

fn run<C: Clock>(
    app: AppHandle,
    rx: Receiver<Control>,
    clock: C,
    wpm: u32,
    highlight: Highlight,
    status: Arc<Mutex<Status>>,
) {
    let mut engine = Engine::new(clock, wpm);
    let mut script: Option<ScriptPayload> = None;
    let mut last: Option<Snapshot> = None;
    let mut mode = highlight;

    loop {
        let wait = engine
            .next_deadline()
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(IDLE_WAIT);
        match rx.recv_timeout(wait) {
            Ok(Control::Start(payload)) => {
                engine.load(payload.tokens.clone());
                engine.start();
                let _ = app.emit_to(prompter::LABEL, SCRIPT_EVENT, &payload);
                script = Some(payload);
                last = None;
            }
            Ok(Control::SetLines(lines)) => engine.set_lines(lines),
            Ok(Control::Act(action)) => match action {
                Action::Pause => engine.pause(),
                Action::Resume => engine.resume(),
                Action::Restart => engine.restart(),
                Action::Stop => engine.stop(),
                Action::Prev => engine.previous(),
                Action::Next => engine.next(),
            },
            Ok(Control::SetWpm(wpm)) => engine.set_wpm(wpm),
            Ok(Control::SetHighlight(highlight)) => mode = highlight,
            Ok(Control::Resend) => {
                if let Some(payload) = &script {
                    let _ = app.emit_to(prompter::LABEL, SCRIPT_EVENT, payload);
                }
                last = None;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        engine.advance();
        let snapshot = engine.snapshot();
        emit_changes(&app, mode, last, snapshot);
        publish(&status, &script, mode, snapshot);
        last = Some(snapshot);
    }
}

/// Mirrors the snapshot for readers on other threads. Writes only when something changed.
fn publish(status: &Mutex<Status>, script: &Option<ScriptPayload>, mode: Highlight, snapshot: Snapshot) {
    let next = Status {
        state: snapshot.state,
        title: script.as_ref().map(|s| s.title.clone()).unwrap_or_default(),
        elapsed: snapshot.elapsed,
        total: snapshot.total,
        highlight: mode,
    };
    let mut current = lock(status);
    if *current != next {
        *current = next;
    }
}

fn emit_changes(app: &AppHandle, mode: Highlight, last: Option<Snapshot>, snapshot: Snapshot) {
    let state_changed = last.is_none_or(|l| l.state != snapshot.state || l.count != snapshot.count);
    let position_changed = last.is_none_or(|l| {
        let word_moved = mode == Highlight::Word && l.word != snapshot.word;
        word_moved
            || l.state != snapshot.state
            || l.line != snapshot.line
            || l.section != snapshot.section
            || l.pause != snapshot.pause
            || l.elapsed.floor() != snapshot.elapsed.floor()
    });
    if state_changed {
        let payload = StatePayload {
            state: snapshot.state,
            count: (snapshot.state == StateKind::Countdown).then_some(snapshot.count),
        };
        for label in LISTENERS {
            let _ = app.emit_to(label, STATE_EVENT, payload);
        }
    }
    if position_changed {
        let payload = PositionPayload {
            word: snapshot.word,
            line: snapshot.line,
            section: snapshot.section,
            elapsed: snapshot.elapsed,
            total: snapshot.total,
            pause: snapshot.pause,
        };
        for label in LISTENERS {
            let _ = app.emit_to(label, POSITION_EVENT, payload);
        }
    }
}
