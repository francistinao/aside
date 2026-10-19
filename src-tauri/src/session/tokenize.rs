//! Turns a script's markdown into the tokens the timing engine reads.

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

/// What a word ends with. Soft punctuation adds a short beat, hard punctuation a longer one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Punctuation {
    None,
    Soft,
    Hard,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Token {
    Word {
        text: String,
        #[serde(skip)]
        letters: u32,
        bold: bool,
        italic: bool,
        #[serde(skip)]
        ends: Punctuation,
    },
    ParagraphEnd,
    Heading {
        text: String,
        level: u8,
    },
    Pause {
        seconds: f64,
    },
}

impl Token {
    pub fn is_word(&self) -> bool {
        matches!(self, Token::Word { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    /// The first H1, if there is one.
    pub title: Option<String>,
    pub tokens: Vec<Token>,
}

pub const MIN_PAUSE: f64 = 0.5;
pub const MAX_PAUSE: f64 = 10.0;

pub fn tokenize(markdown: &str) -> Script {
    let mut builder = Builder::default();
    for event in Parser::new(markdown) {
        builder.event(event);
    }
    builder.finish()
}

#[derive(Default)]
struct Builder {
    tokens: Vec<Token>,
    title: Option<String>,
    bold: u32,
    italic: u32,
    heading: Option<(u8, String)>,
    word: String,
    word_has_letters: bool,
    block_has_words: bool,
}

impl Builder {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                self.flush_word();
                self.heading = Some((heading_level(level), String::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, text)) = self.heading.take() {
                    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    if level == 1 && self.title.is_none() && !text.is_empty() {
                        self.title = Some(text.clone());
                    }
                    self.tokens.push(Token::Heading { text, level });
                }
            }
            Event::End(TagEnd::Paragraph | TagEnd::Item) => self.end_block(),
            Event::Start(Tag::Strong) => self.bold += 1,
            Event::End(TagEnd::Strong) => self.bold = self.bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => self.italic += 1,
            Event::End(TagEnd::Emphasis) => self.italic = self.italic.saturating_sub(1),
            Event::Text(text) | Event::Code(text) => self.text(&text),
            Event::SoftBreak | Event::HardBreak => self.text(" "),
            _ => {}
        }
    }

    fn finish(mut self) -> Script {
        self.end_block();
        Script {
            title: self.title,
            tokens: self.tokens,
        }
    }

    fn text(&mut self, text: &str) {
        if let Some((_, heading)) = self.heading.as_mut() {
            heading.push_str(text);
            return;
        }
        let mut rest = text;
        while let Some(start) = rest.find("{pause") {
            match parse_pause(&rest[start..]) {
                Some((seconds, len)) => {
                    self.words(&rest[..start]);
                    self.flush_word();
                    self.tokens.push(Token::Pause { seconds });
                    rest = &rest[start + len..];
                }
                None => {
                    // Malformed marker: keep it as plain text and look further on.
                    let cut = start + "{pause".len();
                    self.words(&rest[..cut]);
                    rest = &rest[cut..];
                }
            }
        }
        self.words(rest);
        self.flush_word();
    }

    /// Splits on UAX 29 boundaries. Whitespace ends a word, punctuation attaches to the word
    /// before it, and leading punctuation waits for the word after it.
    fn words(&mut self, text: &str) {
        for segment in text.split_word_bounds() {
            if segment.chars().all(char::is_whitespace) {
                self.flush_word();
                continue;
            }
            let wordy = segment.chars().any(char::is_alphanumeric);
            if wordy && self.word_has_letters {
                self.flush_word();
            }
            self.word.push_str(segment);
            if wordy {
                self.word_has_letters = true;
            }
        }
    }

    fn flush_word(&mut self) {
        if self.word.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.word);
        self.word_has_letters = false;
        let letters = text
            .graphemes(true)
            .filter(|g| g.chars().any(char::is_alphanumeric))
            .count() as u32;
        self.tokens.push(Token::Word {
            ends: ending(&text),
            text,
            letters,
            bold: self.bold > 0,
            italic: self.italic > 0,
        });
        self.block_has_words = true;
    }

    fn end_block(&mut self) {
        self.flush_word();
        if self.block_has_words {
            self.tokens.push(Token::ParagraphEnd);
            self.block_has_words = false;
        }
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// The trailing punctuation run decides the beat. Quotes and brackets are looked through.
fn ending(word: &str) -> Punctuation {
    let mut result = Punctuation::None;
    for grapheme in word.graphemes(true).rev() {
        let Some(c) = grapheme.chars().next() else {
            continue;
        };
        if c.is_alphanumeric() {
            break;
        }
        match c {
            '.' | '?' | '!' | '\u{3002}' | '\u{FF1F}' | '\u{FF01}' => return Punctuation::Hard,
            ',' | ';' | ':' | '\u{3001}' | '\u{FF0C}' | '\u{FF1B}' | '\u{FF1A}' => {
                result = Punctuation::Soft;
            }
            _ => {}
        }
    }
    result
}

/// Parses `{pause 2s}` or `{pause 1.5s}` at the start of `text`. Returns the seconds, clamped,
/// and how many bytes the marker spans.
fn parse_pause(text: &str) -> Option<(f64, usize)> {
    let rest = text.strip_prefix("{pause")?;
    let trimmed = rest.trim_start();
    if trimmed.len() == rest.len() {
        return None;
    }
    let digits = trimmed
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit() || *c == '.')
        .map(|(i, c)| i + c.len_utf8())
        .last()?;
    let seconds: f64 = trimmed[..digits].parse().ok()?;
    let after = trimmed[digits..].strip_prefix("s}")?;
    let len = text.len() - after.len();
    Some((seconds.clamp(MIN_PAUSE, MAX_PAUSE), len))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str) -> Token {
        Token::Word {
            text: text.to_string(),
            letters: text
                .graphemes(true)
                .filter(|g| g.chars().any(char::is_alphanumeric))
                .count() as u32,
            bold: false,
            italic: false,
            ends: ending(text),
        }
    }

    fn texts(tokens: &[Token]) -> Vec<String> {
        tokens
            .iter()
            .map(|t| match t {
                Token::Word { text, .. } => text.clone(),
                Token::ParagraphEnd => "<p>".into(),
                Token::Heading { text, level } => format!("<h{level} {text}>"),
                Token::Pause { seconds } => format!("<pause {seconds}>"),
            })
            .collect()
    }

    #[test]
    fn plain_text_becomes_words_with_punctuation_attached() {
        let script = tokenize("Now click the settings cog. Then wait, please.");
        assert_eq!(
            script.tokens,
            vec![
                word("Now"),
                word("click"),
                word("the"),
                word("settings"),
                word("cog."),
                word("Then"),
                word("wait,"),
                word("please."),
                Token::ParagraphEnd,
            ]
        );
        assert_eq!(script.title, None);
    }

    #[test]
    fn punctuation_classes_and_letter_counts() {
        let script = tokenize("Hello, world; wait: go. Why? Yes! \"Quoted.\" (aside) ...");
        let ends: Vec<Punctuation> = script
            .tokens
            .iter()
            .filter_map(|t| match t {
                Token::Word { ends, .. } => Some(*ends),
                _ => None,
            })
            .collect();
        assert_eq!(
            ends,
            vec![
                Punctuation::Soft,
                Punctuation::Soft,
                Punctuation::Soft,
                Punctuation::Hard,
                Punctuation::Hard,
                Punctuation::Hard,
                Punctuation::Hard,
                Punctuation::None,
                Punctuation::Hard,
            ]
        );
        assert_eq!(texts(&script.tokens)[6], "\"Quoted.\"");
        assert_eq!(texts(&script.tokens)[8], "...");
        let letters: Vec<u32> = script
            .tokens
            .iter()
            .filter_map(|t| match t {
                Token::Word { letters, .. } => Some(*letters),
                _ => None,
            })
            .collect();
        assert_eq!(letters, vec![5, 5, 4, 2, 3, 3, 6, 5, 0]);
    }

    #[test]
    fn bold_and_italic_are_flags() {
        let script = tokenize("Click **Save now** and *wait*.");
        let flags: Vec<(String, bool, bool)> = script
            .tokens
            .iter()
            .filter_map(|t| match t {
                Token::Word { text, bold, italic, .. } => Some((text.clone(), *bold, *italic)),
                _ => None,
            })
            .collect();
        assert_eq!(
            flags,
            vec![
                ("Click".into(), false, false),
                ("Save".into(), true, false),
                ("now".into(), true, false),
                ("and".into(), false, false),
                ("wait".into(), false, true),
                (".".into(), false, false),
            ]
        );
    }

    #[test]
    fn lists_end_each_item_as_a_paragraph() {
        let script = tokenize("- One\n- Two three\n\n1. First\n2. Second");
        assert_eq!(
            texts(&script.tokens),
            vec!["One", "<p>", "Two", "three", "<p>", "First", "<p>", "Second", "<p>"]
        );
    }

    #[test]
    fn headings_are_tokens_and_the_first_h1_is_the_title() {
        let script = tokenize("# Welcome to Aside\n\nHello.\n\n## How it works\n\nMove.");
        assert_eq!(script.title.as_deref(), Some("Welcome to Aside"));
        assert_eq!(
            texts(&script.tokens),
            vec![
                "<h1 Welcome to Aside>",
                "Hello.",
                "<p>",
                "<h2 How it works>",
                "Move.",
                "<p>"
            ]
        );
    }

    #[test]
    fn valid_pauses_become_tokens_with_clamping() {
        let script = tokenize("Open it. {pause 2s} Now click. {pause 1.5s} Done. {pause 0.1s} Over {pause 99s} out.");
        assert_eq!(
            texts(&script.tokens),
            vec![
                "Open", "it.", "<pause 2>", "Now", "click.", "<pause 1.5>", "Done.", "<pause 0.5>",
                "Over", "<pause 10>", "out.", "<p>"
            ]
        );
    }

    #[test]
    fn malformed_pauses_stay_as_text() {
        let script = tokenize("Wait {pause two} then {pause 2} and {pause} and { pause 2s } fine.");
        assert_eq!(
            texts(&script.tokens),
            vec![
                "Wait", "{pause", "two}", "then", "{pause", "2}", "and", "{pause}", "and", "{", "pause",
                "2s", "}", "fine.", "<p>"
            ]
        );
    }

    #[test]
    fn korean_splits_on_spaces_and_counts_syllables() {
        let script = tokenize("안녕하세요 여러분, 시작합니다.");
        assert_eq!(texts(&script.tokens), vec!["안녕하세요", "여러분,", "시작합니다.", "<p>"]);
        let letters: Vec<u32> = script
            .tokens
            .iter()
            .filter_map(|t| match t {
                Token::Word { letters, .. } => {
                    assert!(*letters > 0);
                    Some(*letters)
                }
                _ => None,
            })
            .collect();
        assert_eq!(letters, vec![5, 3, 5]);
        assert!(matches!(script.tokens[1], Token::Word { ends: Punctuation::Soft, .. }));
        assert!(matches!(script.tokens[2], Token::Word { ends: Punctuation::Hard, .. }));
    }

    #[test]
    fn empty_script_has_no_tokens() {
        assert_eq!(tokenize(""), Script { title: None, tokens: vec![] });
        assert_eq!(tokenize("\n\n   \n").tokens, vec![]);
    }

    #[test]
    fn headings_only_gives_only_heading_tokens() {
        let script = tokenize("# One\n\n## Two\n\n### Three");
        assert_eq!(texts(&script.tokens), vec!["<h1 One>", "<h2 Two>", "<h3 Three>"]);
        assert!(script.tokens.iter().all(|t| !t.is_word()));
    }

    #[test]
    fn other_markdown_keeps_its_text_only() {
        let script = tokenize("See [the docs](https://example.com) and `code` here.\n\n> quoted\n\n---\n");
        assert_eq!(
            texts(&script.tokens),
            vec!["See", "the", "docs", "and", "code", "here.", "<p>", "quoted", "<p>"]
        );
    }

    #[test]
    fn soft_line_breaks_separate_words() {
        let script = tokenize("one\ntwo  \nthree");
        assert_eq!(texts(&script.tokens), vec!["one", "two", "three", "<p>"]);
    }
}
