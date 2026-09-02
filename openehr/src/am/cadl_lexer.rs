//! A tokenizer for the subset of cADL [`crate::am::cadl`] parses.
//!
//! Deliberately richer than [`crate::am::adl_lexer`] — that one is header-only
//! and explicitly carries no string literals, no numbers, no `<`/`>` — because
//! `definition` needs all three: `"deg C"` string literals, `32.0`/`212` number
//! literals (kept as their own token *kinds*, not merged into `Word`, so the
//! parser never has to re-guess whether `at0004` is a code or `32.0` a number
//! from text alone), and `|`/`<`/`>` for interval syntax. Still not a general
//! ADL/cADL/ODIN lexer: no `EMBEDDED_URI`, no `GUID`, no `VARIABLE_ID` — none
//! of those appear in anything [`crate::am::cadl`] implements.

/// One lexical token cADL parsing needs.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Token {
    /// An identifier, keyword, or `id`/`at`/`ac`-coded token — `DV_QUANTITY`,
    /// `matches`, `at0004`, `id1.1`. Which of those it is depends on
    /// context, exactly as the real grammar resolves its `IDENTIFIER` token
    /// against keyword tokens and the coded-token patterns before falling
    /// back to a bare identifier.
    Word(String),
    /// `INTEGER`: a run of digits with no fraction part, kept as text so the
    /// parser decides sign and target type rather than this lexer guessing.
    Integer(String),
    /// `REAL`: digits, a single `.`, digits — never merged with a following
    /// `..` range separator (see [`Lexer::next`]'s own number-scanning).
    Real(String),
    /// `STRING`: the text between a `"..."` pair, with `\"` and `\\`
    /// unescaped. No other escape sequence appears in the fixtures this
    /// parses; a source using one is refused rather than silently
    /// mis-decoded (see [`Lexer::next`]).
    Str(String),
    /// `..`, the range separator — lexed as its own token specifically so a
    /// `REAL` like `32.0` and a range like `32.0..212.0` never collide (see
    /// [`Lexer::next`]'s number scanning for exactly where the split
    /// happens).
    DotDot,
    /// One of `[ ] { } ( ) , ; | < > * = + -`.
    Symbol(char),
}

impl core::fmt::Display for Token {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Word(w) => write!(f, "`{w}`"),
            Self::Integer(n) | Self::Real(n) => write!(f, "`{n}`"),
            Self::Str(s) => write!(f, "\"{s}\""),
            Self::DotDot => write!(f, "`..`"),
            Self::Symbol(s) => write!(f, "`{s}`"),
        }
    }
}

const SYMBOLS: &str = "[]{}(),;|<>*=+-";

pub(super) struct Lexer<'a> {
    source: &'a str,
    rest: &'a str,
}

impl<'a> Lexer<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        Self {
            source,
            rest: source,
        }
    }

    /// The byte offset the lexer has read up to.
    pub(super) fn offset(&self) -> usize {
        self.source.len() - self.rest.len()
    }

    /// Skips whitespace and `-- ...` line comments, both insignificant
    /// anywhere between tokens in ADL.
    pub(super) fn skip_trivia(&mut self) {
        loop {
            self.rest = self.rest.trim_start();
            if let Some(after) = self.rest.strip_prefix("--") {
                let end = after.find('\n').unwrap_or(after.len());
                self.rest = &after[end..];
                continue;
            }
            break;
        }
    }

    /// Reads a `"..."` string literal, `self.rest` positioned just past the
    /// opening `"`. Unescapes `\"` and `\\` only — `STRING_CHAR`'s other
    /// escapes (`\n`, `\t`, `\u`XXXX, …) do not appear in this parser's own
    /// fixtures, and guessing at one would silently misdecode rather than
    /// refuse (`K15.6`).
    ///
    /// # Errors
    ///
    /// `Err(())` at an unterminated string or an escape this lexer does not
    /// recognise; the caller attributes the offset.
    fn read_string(&mut self) -> Result<String, ()> {
        let mut out = String::new();
        loop {
            let mut chars = self.rest.char_indices();
            let (_, c) = chars.next().ok_or(())?;
            match c {
                '"' => {
                    self.rest = &self.rest[1..];
                    return Ok(out);
                }
                '\\' => {
                    let (_, escaped) = chars.next().ok_or(())?;
                    match escaped {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        _ => return Err(()),
                    }
                    self.rest = &self.rest[c.len_utf8() + escaped.len_utf8()..];
                }
                _ => {
                    out.push(c);
                    self.rest = &self.rest[c.len_utf8()..];
                }
            }
        }
    }

    /// Reads `INTEGER E_SUFFIX?` or `INTEGER '.' INTEGER E_SUFFIX?`,
    /// `self.rest` positioned at the first digit. Stops **before** a second
    /// `.` — `32.0..212.0` lexes as `Real("32.0")`, `DotDot`,
    /// `Real("212.0")`, never one run swallowing the range separator,
    /// because a fraction is only consumed when the `.` is itself followed
    /// by another digit, and `..` never is.
    fn read_number(&mut self) -> Token {
        let digits_len = |s: &str| s.bytes().take_while(u8::is_ascii_digit).count();
        let int_len = digits_len(self.rest);
        let (mut end, mut is_real) = (int_len, false);
        if let Some(after_dot) = self.rest[end..].strip_prefix('.')
            && after_dot.starts_with(|c: char| c.is_ascii_digit())
        {
            is_real = true;
            end += 1 + digits_len(after_dot);
        }
        if let Some(after_e) = self.rest[end..].strip_prefix(['e', 'E']) {
            let after_sign = after_e.strip_prefix(['+', '-']).unwrap_or(after_e);
            let exp_digits = digits_len(after_sign);
            if exp_digits > 0 {
                end += (after_e.len() - after_sign.len()) + 1 + exp_digits;
            }
        }
        let text = self.rest[..end].to_owned();
        self.rest = &self.rest[end..];
        if is_real {
            Token::Real(text)
        } else {
            Token::Integer(text)
        }
    }

    pub(super) fn next(&mut self) -> Option<Token> {
        self.skip_trivia();
        let first = self.rest.chars().next()?;
        if first == '"' {
            let start = self.offset();
            self.rest = &self.rest[1..];
            return Some(match self.read_string() {
                Ok(s) => Token::Str(s),
                // Malformed input is still a token, so the parser reports
                // it at a real position rather than the lexer silently
                // stopping — `read_string`'s own error carries nothing
                // else to attribute, so this is as far as this layer goes;
                // the parser's own "unexpected end of input" / generic
                // refusal covers the rest.
                Err(()) => Token::Word(self.source[start..self.offset().max(start + 1)].into()),
            });
        }
        if first.is_ascii_digit() {
            return Some(self.read_number());
        }
        if self.rest.starts_with("..") {
            self.rest = &self.rest[2..];
            return Some(Token::DotDot);
        }
        if SYMBOLS.contains(first) {
            self.rest = &self.rest[first.len_utf8()..];
            return Some(Token::Symbol(first));
        }
        // A word: letters, digits, underscore, and a single `.` when it sits
        // between two word characters — `id1.1`'s own internal dot, never a
        // `..` range separator (already peeled off above) and never a
        // trailing `.` at a word's end.
        let mut end = 0;
        let mut chars = self.rest.char_indices().peekable();
        while let Some(&(i, c)) = chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                end = i + c.len_utf8();
                chars.next();
            } else if c == '.'
                && self.rest[i + 1..]
                    .starts_with(|next: char| next.is_alphanumeric() || next == '_')
            {
                end = i + 1;
                chars.next();
            } else {
                break;
            }
        }
        if end == 0 {
            // An unrecognised character — not whitespace, not a known
            // symbol, not the start of a word. Consumed as its own
            // single-character word so the parser can name it in a refusal
            // rather than this lexer looping forever.
            end = first.len_utf8();
        }
        let word = &self.rest[..end];
        self.rest = &self.rest[end..];
        Some(Token::Word(word.to_owned()))
    }

    /// Peeks at the next token without consuming it.
    pub(super) fn peek(&mut self) -> Option<Token> {
        let checkpoint = self.rest;
        let token = self.next();
        self.rest = checkpoint;
        token
    }

    /// Whether input is exhausted, after skipping trivia.
    pub(super) fn at_end(&mut self) -> bool {
        self.skip_trivia();
        self.rest.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut out = Vec::new();
        while let Some(t) = lexer.next() {
            out.push(t);
        }
        out
    }

    #[test]
    fn a_range_between_two_reals_does_not_swallow_the_dot_dot() {
        assert_eq!(
            tokens("32.0..212.0"),
            vec![
                Token::Real("32.0".to_owned()),
                Token::DotDot,
                Token::Real("212.0".to_owned()),
            ]
        );
    }

    #[test]
    fn an_integer_range_stays_integers() {
        assert_eq!(
            tokens("0..1"),
            vec![
                Token::Integer("0".to_owned()),
                Token::DotDot,
                Token::Integer("1".to_owned()),
            ]
        );
    }

    #[test]
    fn a_dotted_id_code_is_one_word() {
        assert_eq!(tokens("id1.1"), vec![Token::Word("id1.1".to_owned())]);
    }

    #[test]
    fn a_string_literal_unescapes_quotes_and_backslashes() {
        assert_eq!(
            tokens(r#""deg \"C\"\\""#),
            vec![Token::Str("deg \"C\"\\".to_owned())]
        );
    }

    #[test]
    fn comments_and_whitespace_are_skipped_between_any_two_tokens() {
        assert_eq!(
            tokens("ELEMENT -- a comment\n [at0001]"),
            vec![
                Token::Word("ELEMENT".to_owned()),
                Token::Symbol('['),
                Token::Word("at0001".to_owned()),
                Token::Symbol(']'),
            ]
        );
    }
}
