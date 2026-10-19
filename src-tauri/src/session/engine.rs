//! The timing engine: one source of truth for playback and the duration estimate.
//! Rules come from docs/PRODUCT.md. Time comes only from the injected clock.

use std::time::{Duration, Instant};

use serde::Serialize;

use super::tokenize::{Punctuation, Token};

pub trait Clock {
    fn now(&self) -> Instant;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Reading speed limits from docs/PRODUCT.md.
pub const MIN_WPM: u32 = 80;
pub const MAX_WPM: u32 = 220;

const COUNTDOWN_FROM: u8 = 3;
const COUNTDOWN_STEP: Duration = Duration::from_secs(1);
/// Previous goes to the start of the current line when more than this far into it.
const PREVIOUS_RESTART_AFTER: f64 = 1.0;
/// Smallest gap between now and a computed deadline, in seconds.
const MIN_LEAD: f64 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Estimate {
    pub words: u32,
    pub seconds: f64,
}

/// Seconds a token takes at `wpm`.
pub fn duration(token: &Token, wpm: u32) -> f64 {
    let base = 60.0 / f64::from(wpm.max(1));
    match token {
        Token::Word { letters, ends, .. } => {
            let length = (1.0 + 0.04 * (f64::from(*letters) - 5.0)).clamp(0.8, 1.6);
            let beat = match ends {
                Punctuation::None => 0.0,
                Punctuation::Soft => 0.25,
                Punctuation::Hard => 0.6,
            };
            base * (length + beat)
        }
        Token::ParagraphEnd => base,
        Token::Heading { .. } => base * 1.5,
        Token::Pause { seconds } => *seconds,
    }
}

pub fn estimate(tokens: &[Token], wpm: u32) -> Estimate {
    Estimate {
        words: tokens.iter().filter(|t| t.is_word()).count() as u32,
        seconds: tokens.iter().map(|t| duration(t, wpm)).sum(),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StateKind {
    #[default]
    Idle,
    Countdown,
    Running,
    Paused,
    Ended,
}

/// Everything the overlay and the Library need to draw one moment of the session.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snapshot {
    pub state: StateKind,
    /// Countdown digit, 0 outside the countdown.
    pub count: u8,
    /// Index among word tokens, None during headings, pauses and paragraph ends.
    pub word: Option<u32>,
    pub line: u32,
    /// Number of headings passed, 0 before the first.
    pub section: u32,
    /// Script seconds at the current speed.
    pub elapsed: f64,
    pub total: f64,
    /// Whole seconds left in a pause marker.
    pub pause: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
enum State {
    Idle,
    Countdown { count: u8, deadline: Instant },
    Running { cursor: usize, started: Instant },
    Paused { cursor: usize, offset: f64 },
    Ended,
}

pub struct Engine<C: Clock> {
    clock: C,
    tokens: Vec<Token>,
    wpm: u32,
    durations: Vec<f64>,
    /// Script seconds before each token.
    starts: Vec<f64>,
    total: f64,
    /// Token index of each word.
    word_tokens: Vec<usize>,
    /// For each token, the index of the last word at or before it.
    word_before: Vec<Option<u32>>,
    /// For each token, how many headings sit at or before it.
    headings_before: Vec<u32>,
    /// Inclusive word ranges per visual line. Empty means one line for the whole script.
    lines: Vec<(u32, u32)>,
    word_line: Vec<u32>,
    state: State,
}

impl<C: Clock> Engine<C> {
    pub fn new(clock: C, wpm: u32) -> Self {
        Self {
            clock,
            tokens: Vec::new(),
            wpm: wpm.max(1),
            durations: Vec::new(),
            starts: Vec::new(),
            total: 0.0,
            word_tokens: Vec::new(),
            word_before: Vec::new(),
            headings_before: Vec::new(),
            lines: Vec::new(),
            word_line: Vec::new(),
            state: State::Idle,
        }
    }

    /// Replaces the script. The session goes back to Idle and lines are cleared.
    pub fn load(&mut self, tokens: Vec<Token>) {
        self.tokens = tokens;
        self.word_tokens = self
            .tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_word())
            .map(|(i, _)| i)
            .collect();
        let mut last_word = None;
        let mut word_count = 0;
        let mut headings = 0;
        self.word_before = Vec::with_capacity(self.tokens.len());
        self.headings_before = Vec::with_capacity(self.tokens.len());
        for token in &self.tokens {
            if token.is_word() {
                last_word = Some(word_count);
                word_count += 1;
            }
            if matches!(token, Token::Heading { .. }) {
                headings += 1;
            }
            self.word_before.push(last_word);
            self.headings_before.push(headings);
        }
        self.lines.clear();
        self.word_line = vec![0; self.word_tokens.len()];
        self.rebuild_durations();
        self.state = State::Idle;
    }

    pub fn wpm(&self) -> u32 {
        self.wpm
    }

    pub fn total(&self) -> f64 {
        self.total
    }

    pub fn words(&self) -> u32 {
        self.word_tokens.len() as u32
    }

    fn rebuild_durations(&mut self) {
        self.durations = self.tokens.iter().map(|t| duration(t, self.wpm)).collect();
        self.starts = Vec::with_capacity(self.durations.len());
        let mut sum = 0.0;
        for d in &self.durations {
            self.starts.push(sum);
            sum += d;
        }
        self.total = sum;
    }

    /// Visual lines as inclusive word index ranges, in order. Gaps belong to the line before.
    pub fn set_lines(&mut self, lines: Vec<(u32, u32)>) {
        let words = self.word_tokens.len() as u32;
        self.lines = lines
            .into_iter()
            .filter(|(start, end)| start <= end && *end < words)
            .collect();
        self.word_line = vec![0; words as usize];
        for (line, (start, _)) in self.lines.iter().enumerate() {
            let next_start = self
                .lines
                .get(line + 1)
                .map(|(s, _)| *s as usize)
                .unwrap_or(words as usize);
            for slot in self.word_line.iter_mut().take(next_start).skip(*start as usize) {
                *slot = line as u32;
            }
        }
    }

    pub fn start(&mut self) {
        if self.tokens.is_empty() {
            self.state = State::Ended;
            return;
        }
        self.state = State::Countdown {
            count: COUNTDOWN_FROM,
            deadline: self.clock.now() + COUNTDOWN_STEP,
        };
    }

    pub fn stop(&mut self) {
        self.state = State::Idle;
    }

    pub fn restart(&mut self) {
        self.start();
    }

    pub fn pause(&mut self) {
        if let State::Running { cursor, started } = self.state {
            let offset = self.offset_in(cursor, started);
            self.state = State::Paused { cursor, offset };
        }
    }

    pub fn resume(&mut self) {
        if let State::Paused { cursor, offset } = self.state {
            self.state = State::Running {
                cursor,
                started: self.clock.now() - Duration::from_secs_f64(offset),
            };
        }
    }

    /// Changes speed. The rest of the current token scales with the new duration.
    pub fn set_wpm(&mut self, wpm: u32) {
        let wpm = wpm.max(1);
        if wpm == self.wpm {
            return;
        }
        let fraction = match self.state {
            State::Running { cursor, started } => Some((cursor, self.fraction_in(cursor, started))),
            State::Paused { cursor, offset } => Some((cursor, self.fraction_of(cursor, offset))),
            _ => None,
        };
        self.wpm = wpm;
        self.rebuild_durations();
        if let Some((cursor, fraction)) = fraction {
            let offset = fraction * self.durations.get(cursor).copied().unwrap_or(0.0);
            self.state = match self.state {
                State::Running { .. } => State::Running {
                    cursor,
                    started: self.clock.now() - Duration::from_secs_f64(offset),
                },
                _ => State::Paused { cursor, offset },
            };
        }
    }

    pub fn previous(&mut self) {
        let Some(cursor) = self.cursor() else {
            return;
        };
        let line = self.line_of(cursor);
        let into = self.elapsed() - self.line_start_time(line);
        let target = if into > PREVIOUS_RESTART_AFTER {
            line
        } else {
            line.saturating_sub(1)
        };
        self.seek_line(target);
    }

    pub fn next(&mut self) {
        let Some(cursor) = self.cursor() else {
            return;
        };
        let line = self.line_of(cursor);
        if (line as usize + 1) < self.line_count() {
            self.seek_line(line + 1);
        } else {
            self.state = State::Ended;
        }
    }

    /// Moves the state machine as far as the clock allows.
    pub fn advance(&mut self) {
        let now = self.clock.now();
        loop {
            match self.state {
                State::Countdown { count, deadline } if now >= deadline => {
                    if count <= 1 {
                        self.state = State::Running {
                            cursor: 0,
                            started: deadline,
                        };
                    } else {
                        self.state = State::Countdown {
                            count: count - 1,
                            deadline: deadline + COUNTDOWN_STEP,
                        };
                    }
                }
                State::Running { cursor, started } => {
                    let Some(&dur) = self.durations.get(cursor) else {
                        self.state = State::Ended;
                        continue;
                    };
                    let ends = started + Duration::from_secs_f64(dur);
                    if now < ends {
                        return;
                    }
                    if cursor + 1 >= self.tokens.len() {
                        self.state = State::Ended;
                        return;
                    }
                    self.state = State::Running {
                        cursor: cursor + 1,
                        started: ends,
                    };
                }
                _ => return,
            }
        }
    }

    /// When something next changes on its own, if anything.
    pub fn next_deadline(&self) -> Option<Instant> {
        match self.state {
            State::Countdown { deadline, .. } => Some(deadline),
            State::Running { cursor, started } => {
                let dur = self.durations.get(cursor).copied()?;
                let token_end = started + Duration::from_secs_f64(dur);
                let offset = self.offset_in(cursor, started);
                let mut deadline = token_end;
                // Whole seconds tick the footer: the pause countdown and the elapsed time.
                let next_second_in_token = if matches!(self.tokens.get(cursor), Some(Token::Pause { .. })) {
                    let remaining = dur - offset;
                    let next = remaining.ceil() - 1.0;
                    (next > 0.0).then_some(dur - next)
                } else {
                    None
                };
                let elapsed = self.starts.get(cursor).copied().unwrap_or(0.0) + offset;
                let next_elapsed_second = elapsed.floor() + 1.0 - self.starts.get(cursor).copied().unwrap_or(0.0);
                for candidate in [next_second_in_token, Some(next_elapsed_second)].into_iter().flatten() {
                    // A microsecond of lead keeps float rounding from producing a deadline that is now.
                    if candidate > offset + MIN_LEAD && candidate < dur {
                        let at = started + Duration::from_secs_f64(candidate);
                        if at < deadline {
                            deadline = at;
                        }
                    }
                }
                Some(deadline)
            }
            _ => None,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let (state, count) = match self.state {
            State::Idle => (StateKind::Idle, 0),
            State::Countdown { count, .. } => (StateKind::Countdown, count),
            State::Running { .. } => (StateKind::Running, 0),
            State::Paused { .. } => (StateKind::Paused, 0),
            State::Ended => (StateKind::Ended, 0),
        };
        let cursor = self.cursor();
        let word = cursor.and_then(|c| match self.tokens.get(c) {
            Some(Token::Word { .. }) => self.word_before.get(c).copied().flatten(),
            _ => None,
        });
        let line = cursor.map(|c| self.line_of(c)).unwrap_or(0);
        let section = match self.state {
            State::Ended => self.headings_before.last().copied().unwrap_or(0),
            _ => cursor
                .and_then(|c| self.headings_before.get(c).copied())
                .unwrap_or(0),
        };
        let pause = cursor.and_then(|c| match self.tokens.get(c) {
            Some(Token::Pause { .. }) => {
                let remaining = self.durations[c] - self.offset();
                Some(remaining.ceil().max(0.0) as u32)
            }
            _ => None,
        });
        Snapshot {
            state,
            count,
            word,
            line,
            section,
            elapsed: self.elapsed(),
            total: self.total,
            pause,
        }
    }

    pub fn elapsed(&self) -> f64 {
        match self.state {
            State::Idle | State::Countdown { .. } => 0.0,
            State::Running { cursor, started } => {
                self.starts.get(cursor).copied().unwrap_or(0.0) + self.offset_in(cursor, started)
            }
            State::Paused { cursor, offset } => self.starts.get(cursor).copied().unwrap_or(0.0) + offset,
            State::Ended => self.total,
        }
    }

    fn cursor(&self) -> Option<usize> {
        match self.state {
            State::Running { cursor, .. } | State::Paused { cursor, .. } => Some(cursor),
            _ => None,
        }
    }

    /// Seconds into the current token, never past its end.
    fn offset(&self) -> f64 {
        match self.state {
            State::Running { cursor, started } => self.offset_in(cursor, started),
            State::Paused { offset, .. } => offset,
            _ => 0.0,
        }
    }

    fn offset_in(&self, cursor: usize, started: Instant) -> f64 {
        let dur = self.durations.get(cursor).copied().unwrap_or(0.0);
        self.clock
            .now()
            .saturating_duration_since(started)
            .as_secs_f64()
            .min(dur)
    }

    fn fraction_in(&self, cursor: usize, started: Instant) -> f64 {
        self.fraction_of(cursor, self.offset_in(cursor, started))
    }

    fn fraction_of(&self, cursor: usize, offset: f64) -> f64 {
        let dur = self.durations.get(cursor).copied().unwrap_or(0.0);
        if dur > 0.0 { (offset / dur).clamp(0.0, 1.0) } else { 0.0 }
    }

    fn line_count(&self) -> usize {
        self.lines.len().max(1)
    }

    fn line_of(&self, cursor: usize) -> u32 {
        let word = self
            .word_before
            .get(cursor)
            .copied()
            .flatten()
            .unwrap_or(0);
        self.word_line.get(word as usize).copied().unwrap_or(0)
    }

    fn line_first_token(&self, line: u32) -> usize {
        let first_word = self.lines.get(line as usize).map(|(start, _)| *start).unwrap_or(0);
        self.word_tokens.get(first_word as usize).copied().unwrap_or(0)
    }

    fn line_start_time(&self, line: u32) -> f64 {
        self.starts
            .get(self.line_first_token(line))
            .copied()
            .unwrap_or(0.0)
    }

    fn seek_line(&mut self, line: u32) {
        let cursor = self.line_first_token(line);
        self.state = match self.state {
            State::Paused { .. } => State::Paused { cursor, offset: 0.0 },
            _ => State::Running {
                cursor,
                started: self.clock.now(),
            },
        };
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::session::tokenize::tokenize;

    #[derive(Clone)]
    struct FakeClock(Rc<Cell<Instant>>);

    impl FakeClock {
        fn new() -> Self {
            Self(Rc::new(Cell::new(Instant::now())))
        }

        fn advance(&self, seconds: f64) {
            self.0.set(self.0.get() + Duration::from_secs_f64(seconds));
        }

        /// Jumps to the engine's next deadline and returns how far it moved.
        fn step_to<C: Clock>(&self, engine: &Engine<C>) -> f64 {
            let deadline = engine.next_deadline().expect("a deadline");
            let delta = deadline.saturating_duration_since(self.0.get()).as_secs_f64();
            self.0.set(deadline);
            delta
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> Instant {
            self.0.get()
        }
    }

    fn word(text: &str, letters: u32, ends: Punctuation) -> Token {
        Token::Word {
            text: text.into(),
            letters,
            bold: false,
            italic: false,
            ends,
        }
    }

    fn engine_with(markdown: &str, wpm: u32) -> (FakeClock, Engine<FakeClock>) {
        let clock = FakeClock::new();
        let mut engine = Engine::new(clock.clone(), wpm);
        engine.load(tokenize(markdown).tokens);
        (clock, engine)
    }

    /// Starts and runs through the countdown so the first word is current.
    fn running(markdown: &str, wpm: u32) -> (FakeClock, Engine<FakeClock>) {
        let (clock, mut engine) = engine_with(markdown, wpm);
        engine.start();
        clock.advance(3.0);
        engine.advance();
        assert_eq!(engine.snapshot().state, StateKind::Running);
        (clock, engine)
    }

    const BASE: f64 = 0.4; // 150 wpm

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn every_duration_rule() {
        assert!(close(duration(&word("hello", 5, Punctuation::None), 150), BASE));
        assert!(close(duration(&word("a", 1, Punctuation::None), 150), BASE * 0.84));
        assert!(close(duration(&word("x", 30, Punctuation::None), 150), BASE * 1.6));
        assert!(close(duration(&word("ok", 2, Punctuation::None), 150), BASE * 0.88));
        assert!(close(duration(&word("wait,", 4, Punctuation::Soft), 150), BASE * (0.96 + 0.25)));
        assert!(close(duration(&word("go.", 2, Punctuation::Hard), 150), BASE * (0.88 + 0.6)));
        assert!(close(duration(&Token::ParagraphEnd, 150), BASE));
        assert!(close(duration(&Token::Heading { text: "x".into(), level: 2 }, 150), BASE * 1.5));
        assert!(close(duration(&Token::Pause { seconds: 2.0 }, 150), 2.0));
        assert!(close(duration(&Token::Pause { seconds: 2.0 }, 80), 2.0));
        assert!(close(duration(&word("hello", 5, Punctuation::None), 300), 0.2));
    }

    #[test]
    fn estimate_equals_total_playback_time() {
        let sample = include_str!("../../../assets/sample-script.md");
        let script = tokenize(sample);
        let estimate = estimate(&script.tokens, 150);
        assert_eq!(estimate.words, script.tokens.iter().filter(|t| t.is_word()).count() as u32);
        let (clock, mut engine) = running(sample, 150);
        let mut played = 0.0;
        while engine.snapshot().state == StateKind::Running {
            played += clock.step_to(&engine);
            engine.advance();
            assert!(played < 1000.0, "never ended");
        }
        assert_eq!(engine.snapshot().state, StateKind::Ended);
        assert!(close(played, estimate.seconds), "played {played}, estimated {}", estimate.seconds);
        assert!(close(engine.snapshot().elapsed, estimate.seconds));
        assert!(close(engine.total(), estimate.seconds));
    }

    #[test]
    fn countdown_sequence_then_running() {
        let (clock, mut engine) = engine_with("One two three.", 150);
        assert_eq!(engine.snapshot().state, StateKind::Idle);
        engine.start();
        let s = engine.snapshot();
        assert_eq!((s.state, s.count, s.word, s.elapsed), (StateKind::Countdown, 3, None, 0.0));
        clock.advance(1.0);
        engine.advance();
        assert_eq!(engine.snapshot().count, 2);
        clock.advance(1.0);
        engine.advance();
        assert_eq!(engine.snapshot().count, 1);
        clock.advance(1.0);
        engine.advance();
        let s = engine.snapshot();
        assert_eq!((s.state, s.count, s.word, s.line), (StateKind::Running, 0, Some(0), 0));
        assert!(close(s.elapsed, 0.0));
    }

    #[test]
    fn words_change_at_their_boundaries() {
        // "One" has three letters: factor 0.92.
        let (clock, mut engine) = running("One two three.", 150);
        clock.advance(BASE * 0.92 - 0.01);
        engine.advance();
        assert_eq!(engine.snapshot().word, Some(0));
        clock.advance(0.02);
        engine.advance();
        assert_eq!(engine.snapshot().word, Some(1));
        assert!(close(engine.snapshot().elapsed, BASE * 0.92 + 0.01));
    }

    #[test]
    fn pause_and_resume_keep_the_position() {
        let (clock, mut engine) = running("Hello there friend.", 150);
        clock.advance(0.15);
        engine.advance();
        engine.pause();
        let paused = engine.snapshot();
        assert_eq!((paused.state, paused.word), (StateKind::Paused, Some(0)));
        clock.advance(5.0);
        engine.advance();
        assert_eq!(engine.snapshot(), paused);
        assert!(engine.next_deadline().is_none());
        engine.resume();
        assert_eq!(engine.snapshot().state, StateKind::Running);
        assert!(close(engine.snapshot().elapsed, 0.15));
        clock.advance(BASE - 0.15 + 0.01);
        engine.advance();
        assert_eq!(engine.snapshot().word, Some(1));
    }

    #[test]
    fn speed_change_mid_word_keeps_the_fraction() {
        let (clock, mut engine) = running("Hello there friend.", 150);
        clock.advance(0.2); // halfway through a 0.4 s word
        engine.advance();
        engine.set_wpm(300); // the word is now 0.2 s, half of it is left: 0.1 s
        let s = engine.snapshot();
        assert_eq!(s.word, Some(0));
        assert!(close(s.elapsed, 0.1));
        assert!(close(s.total, estimate(&tokenize("Hello there friend.").tokens, 300).seconds));
        clock.advance(0.09);
        engine.advance();
        assert_eq!(engine.snapshot().word, Some(0));
        clock.advance(0.02);
        engine.advance();
        assert_eq!(engine.snapshot().word, Some(1));
        // Paused scaling works the same way.
        engine.pause();
        engine.set_wpm(150);
        assert_eq!(engine.snapshot().state, StateKind::Paused);
        assert!(close(engine.snapshot().elapsed, BASE + 0.01 * 2.0));
    }

    #[test]
    fn previous_and_next_at_both_edges() {
        // One-letter words take 0.336 s each. The last line is four words, longer than a second.
        let (clock, mut engine) = running("a b c d e f g h", 150);
        engine.set_lines(vec![(0, 1), (2, 3), (4, 7)]);
        // Next from line 0 goes to the start of line 1 at word 2.
        engine.next();
        let s = engine.snapshot();
        assert_eq!((s.word, s.line), (Some(2), 1));
        assert!(close(s.elapsed, 2.0 * BASE * 0.84));
        // Previous within the first second returns to the previous line.
        clock.advance(0.3);
        engine.advance();
        engine.previous();
        assert_eq!(engine.snapshot().line, 0);
        assert_eq!(engine.snapshot().word, Some(0));
        // Previous at line 0 stays at its start.
        engine.previous();
        assert_eq!((engine.snapshot().line, engine.snapshot().word), (0, Some(0)));
        // Previous after more than a second goes to the line start, not further back.
        engine.next();
        engine.next();
        assert_eq!(engine.snapshot().line, 2);
        clock.advance(1.2);
        engine.advance();
        engine.previous();
        assert_eq!((engine.snapshot().line, engine.snapshot().word), (2, Some(4)));
        // Next on the last line ends the script.
        engine.next();
        assert_eq!(engine.snapshot().state, StateKind::Ended);
        assert_eq!(engine.snapshot().word, None);
    }

    #[test]
    fn previous_and_next_work_while_paused() {
        let (_clock, mut engine) = running("a b c d", 150);
        engine.set_lines(vec![(0, 1), (2, 3)]);
        engine.pause();
        engine.next();
        let s = engine.snapshot();
        assert_eq!((s.state, s.line, s.word), (StateKind::Paused, 1, Some(2)));
        engine.previous();
        assert_eq!((engine.snapshot().state, engine.snapshot().line), (StateKind::Paused, 0));
    }

    #[test]
    fn restart_goes_back_to_the_countdown() {
        let (clock, mut engine) = running("One two.", 150);
        clock.advance(0.5);
        engine.advance();
        engine.restart();
        let s = engine.snapshot();
        assert_eq!((s.state, s.count, s.word), (StateKind::Countdown, 3, None));
        assert!(close(s.elapsed, 0.0));
    }

    #[test]
    fn stop_goes_idle() {
        let (_clock, mut engine) = running("One two.", 150);
        engine.stop();
        assert_eq!(engine.snapshot().state, StateKind::Idle);
        assert!(engine.next_deadline().is_none());
    }

    #[test]
    fn end_state_sticks() {
        let (clock, mut engine) = running("One.", 150);
        clock.advance(10.0);
        engine.advance();
        let s = engine.snapshot();
        assert_eq!((s.state, s.word, s.pause), (StateKind::Ended, None, None));
        assert!(close(s.elapsed, s.total));
        clock.advance(10.0);
        engine.advance();
        assert_eq!(engine.snapshot(), s);
        assert!(engine.next_deadline().is_none());
    }

    #[test]
    fn empty_script_ends_at_once() {
        let (_clock, mut engine) = engine_with("", 150);
        engine.start();
        assert_eq!(engine.snapshot().state, StateKind::Ended);
    }

    #[test]
    fn pause_marker_counts_down_whole_seconds() {
        let (clock, mut engine) = running("Go. {pause 2.5s} Now.", 150);
        let go = BASE * (0.88 + 0.6);
        clock.advance(go + 0.001);
        engine.advance();
        let s = engine.snapshot();
        assert_eq!((s.word, s.pause), (None, Some(3)));
        clock.advance(0.6);
        engine.advance();
        assert_eq!(engine.snapshot().pause, Some(2), "1.9 s left");
        clock.advance(1.0);
        engine.advance();
        assert_eq!(engine.snapshot().pause, Some(1), "0.9 s left");
        clock.advance(0.9);
        engine.advance();
        let s = engine.snapshot();
        assert_eq!((s.word, s.pause), (Some(1), None));
    }

    #[test]
    fn deadlines_visit_every_pause_second() {
        let (clock, mut engine) = running("Go. {pause 2.5s} Now.", 150);
        let mut seen = Vec::new();
        for _ in 0..50 {
            clock.step_to(&engine);
            engine.advance();
            let s = engine.snapshot();
            if let Some(p) = s.pause
                && seen.last() != Some(&p)
            {
                seen.push(p);
            }
            if s.word == Some(1) {
                break;
            }
        }
        assert_eq!(seen, vec![3, 2, 1]);
    }

    #[test]
    fn sections_count_headings_passed() {
        let (clock, mut engine) = running("Intro.\n\n## First\n\nOne.\n\n## Second\n\nTwo.", 150);
        assert_eq!(engine.snapshot().section, 0);
        // Intro. then ParagraphEnd, then the heading.
        clock.advance(BASE * (1.0 + 0.6) + BASE + 0.001);
        engine.advance();
        let s = engine.snapshot();
        assert_eq!((s.section, s.word), (1, None));
        clock.advance(100.0);
        engine.advance();
        assert_eq!((engine.snapshot().state, engine.snapshot().section), (StateKind::Ended, 2));
    }

    #[test]
    fn elapsed_second_is_a_deadline_during_long_words() {
        let (clock, mut engine) = running("Go. {pause 4s} Done.", 150);
        clock.advance(BASE * (0.88 + 0.6) + 0.001);
        engine.advance();
        // Elapsed is about 0.59; the next whole elapsed second (1.0) comes before the pause tick at 1.0.
        let deadline = engine.next_deadline().expect("deadline");
        let delta = deadline.saturating_duration_since(clock.now()).as_secs_f64();
        assert!(delta <= 1.0 - 0.59 + 0.01, "delta {delta}");
    }
}
