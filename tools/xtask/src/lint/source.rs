//! A line lexer for Rust source that separates code from comments.
//!
//! The lexer replaces the content of string and character literals with spaces, so a check of
//! the code never matches the text of a literal.

use std::mem;

/// One line of a Rust file, split into its code and its comments.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct SourceLine {
    /// The code of the line, with literal contents replaced by spaces.
    pub(crate) code: String,
    /// The comment text of the line, with its `//` or `/*` markers.
    pub(crate) comment: String,
}

impl SourceLine {
    /// Returns the comment if the line is a `//` comment with no code, and not a doc comment.
    pub(crate) fn plain_comment(&self) -> Option<&str> {
        let is_doc = self.comment.starts_with("///") && !self.comment.starts_with("////")
            || self.comment.starts_with("//!");
        let is_plain = self.code.trim().is_empty() && self.comment.starts_with("//") && !is_doc;
        is_plain.then_some(self.comment.as_str())
    }

    /// Returns the identifiers and keywords of the code, in sequence.
    pub(crate) fn words(&self) -> impl Iterator<Item = &str> {
        self.code
            .split(|character: char| !is_word_character(character))
            .filter(|word| !word.is_empty())
    }
}

pub(crate) fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Code,
    LineComment,
    BlockComment { depth: usize },
    Literal(Literal),
}

/// The kind of a string literal, which sets its escapes and its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Literal {
    Plain,
    Raw { hashes: usize },
}

/// Splits Rust source into lines of code and comments.
pub(crate) fn lex(source: &str) -> Vec<SourceLine> {
    let mut lexer = Lexer {
        characters: source.chars().collect(),
        position: 0,
        state: State::Code,
        line: SourceLine::default(),
        lines: Vec::new(),
    };
    while let Some(character) = lexer.advance() {
        lexer.step(character);
    }
    lexer.lines.push(lexer.line);
    lexer.lines
}

struct Lexer {
    characters: Vec<char>,
    position: usize,
    state: State,
    line: SourceLine,
    lines: Vec<SourceLine>,
}

impl Lexer {
    fn advance(&mut self) -> Option<char> {
        let character = self.peek(0)?;
        self.position += 1;
        Some(character)
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.characters.get(self.position + offset).copied()
    }

    fn step(&mut self, character: char) {
        if character == '\n' {
            self.lines.push(mem::take(&mut self.line));
            if self.state == State::LineComment {
                self.state = State::Code;
            }
            return;
        }
        match self.state {
            State::Code => self.code(character),
            State::LineComment => self.line.comment.push(character),
            State::BlockComment { depth } => self.block_comment(character, depth),
            State::Literal(literal) => self.literal(character, literal),
        }
    }

    fn code(&mut self, character: char) {
        match (character, self.peek(0)) {
            ('/', Some('/')) => self.state = State::LineComment,
            ('/', Some('*')) => return self.block_comment_start(),
            ('"', _) => self.state = State::Literal(Literal::Plain),
            ('r', _) if self.is_raw_literal_start() => return self.raw_literal_start(),
            ('\'', _) => return self.quote(),
            _ => {}
        }
        let target = match self.state {
            State::Code | State::Literal(_) => &mut self.line.code,
            State::LineComment | State::BlockComment { .. } => &mut self.line.comment,
        };
        target.push(character);
    }

    /// Returns `true` at the `r` of `r"`, `br"` or `cr"`, with any number of `#` before the `"`.
    fn is_raw_literal_start(&self) -> bool {
        let before = |distance: usize| {
            let index = self.position.checked_sub(distance)?;
            self.characters.get(index).copied()
        };
        let is_word_start =
            |previous: Option<char>| previous.is_none_or(|previous| !is_word_character(previous));
        let is_prefixed = matches!(before(2), Some('b' | 'c')) && is_word_start(before(3));
        let hashes = self.count_from(0, |character| character == '#');
        (is_word_start(before(2)) || is_prefixed) && self.peek(hashes) == Some('"')
    }

    fn raw_literal_start(&mut self) {
        self.line.code.push('r');
        let mut hashes = 0;
        while let Some(character) = self.advance() {
            self.line.code.push(character);
            if character == '"' {
                break;
            }
            hashes += 1;
        }
        self.state = State::Literal(Literal::Raw { hashes });
    }

    /// Handles a `'`, which starts a character literal or a lifetime. The content of a character
    /// literal becomes spaces.
    fn quote(&mut self) {
        self.line.code.push('\'');
        let content = match (self.peek(0), self.peek(1)) {
            (Some('\\'), _) => 2 + self.count_from(2, |character| character != '\''),
            (Some(_), Some('\'')) => 1,
            _ => return,
        };
        for _ in 0..content {
            self.advance();
            self.line.code.push(' ');
        }
        if self.advance().is_some() {
            self.line.code.push('\'');
        }
    }

    fn count_from(&self, offset: usize, is_counted: impl Fn(char) -> bool) -> usize {
        self.characters
            .get(self.position + offset..)
            .map_or(0, |rest| {
                rest.iter()
                    .take_while(|character| is_counted(**character))
                    .count()
            })
    }

    fn block_comment_start(&mut self) {
        self.line.comment.push('/');
        if let Some(star) = self.advance() {
            self.line.comment.push(star);
        }
        self.state = State::BlockComment { depth: 0 };
    }

    fn block_comment(&mut self, character: char, depth: usize) {
        self.line.comment.push(character);
        let next_depth = match (character, self.peek(0)) {
            ('*', Some('/')) => depth.checked_sub(1),
            ('/', Some('*')) => Some(depth + 1),
            _ => return,
        };
        if let Some(next) = self.advance() {
            self.line.comment.push(next);
        }
        self.state = next_depth.map_or(State::Code, |remaining| State::BlockComment {
            depth: remaining,
        });
    }

    fn literal(&mut self, character: char, literal: Literal) {
        match (character, literal) {
            ('\\', Literal::Plain) => {
                self.line.code.push(' ');
                if self.peek(0).is_some_and(|next| next != '\n') {
                    self.advance();
                    self.line.code.push(' ');
                }
            }
            ('"', Literal::Plain) => self.close_literal(0),
            ('"', Literal::Raw { hashes }) if self.count_from(0, |next| next == '#') >= hashes => {
                self.close_literal(hashes);
            }
            _ => self.line.code.push(' '),
        }
    }

    fn close_literal(&mut self, hashes: usize) {
        self.line.code.push('"');
        for _ in 0..hashes {
            self.advance();
            self.line.code.push('#');
        }
        self.state = State::Code;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single(source: &str) -> SourceLine {
        let mut lines = lex(source);
        assert_eq!(lines.len(), 1, "{lines:?}");
        lines.remove(0)
    }

    #[test]
    fn lex_separates_code_and_line_comment() {
        let line = single("let x = 1; // done");

        assert_eq!(line.code, "let x = 1; ");
        assert_eq!(line.comment, "// done");
    }

    #[test]
    fn lex_blanks_string_literal_when_it_looks_like_comment() {
        let content = r#"// not \"a\" comment"#;

        let line = single(&format!("let s = \"{content}\"; x"));

        let blank = " ".repeat(content.chars().count());
        assert_eq!(line.code, format!("let s = \"{blank}\"; x"));
        assert!(line.comment.is_empty());
    }

    #[test]
    fn lex_blanks_raw_string_literals() {
        let hashed = single("let s = br#\"unsafe x\"#; // end");
        let prefixed = single(r#"let s = cr"a\"; // end"#);

        let words: Vec<_> = hashed.code.split_whitespace().collect();
        assert_eq!(words, ["let", "s", "=", "br#\"", "\"#;"]);
        assert_eq!(
            (hashed.comment.as_str(), prefixed.comment.as_str()),
            ("// end", "// end")
        );
    }

    #[test]
    fn lex_blanks_char_literals_and_keeps_lifetimes() {
        let line = single(r#"fn f<'a>(x: &'a str) -> [char; 3] { ['"', '\'', '\u{2014}'] } // c"#);

        assert_eq!(
            line.words().collect::<Vec<_>>(),
            ["fn", "f", "a", "x", "a", "str", "char", "3"]
        );
        assert_eq!(line.comment, "// c");
    }

    #[test]
    fn lex_keeps_nested_block_comment_over_lines() {
        let lines = lex("a /* one /* two */\n still */ b");

        assert_eq!(lines[0].code, "a ");
        assert_eq!(lines[0].comment, "/* one /* two */");
        assert_eq!(lines[1].code, " b");
        assert_eq!(lines[1].comment, " still */");
    }

    #[test]
    fn plain_comment_excludes_doc_comments_and_code_lines() {
        let plain = ["// why", "    // why", "//// four"];
        let other = ["/// doc", "//! inner", "x(); // trailing", "let y = 2;"];

        for source in plain {
            assert!(single(source).plain_comment().is_some(), "{source}");
        }
        for source in other {
            assert!(single(source).plain_comment().is_none(), "{source}");
        }
    }
}
