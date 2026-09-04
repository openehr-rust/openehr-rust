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

/// Whether `c` can appear in an ISO8601 date/time/date-time/duration
/// literal, or in one of the four `*_CONSTRAINT_PATTERN` tokens
/// (`base_lexer.g4`) those same four kinds also admit (`A-75`) —
/// `read_iso8601`/`peek_iso8601` read either shape as one raw run and
/// leave classifying it (value or pattern, and which of the four kinds)
/// to the parser. `X`/`x` joins the letter set here for `MONTH_PATTERN`'s
/// (and its siblings') `XX`/`xx` wildcard spelling, alongside `?` for
/// `??`, already present for `DATE_CONSTRAINT_PATTERN`'s own placeholder
/// digits.
fn is_iso8601_char(c: char) -> bool {
    c.is_ascii_digit()
        || matches!(c, '-' | ':' | '+' | '?' | ',')
        || matches!(
            c.to_ascii_uppercase(),
            'T' | 'Z' | 'P' | 'Y' | 'W' | 'D' | 'H' | 'S' | 'M' | 'X'
        )
}

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

    /// The source text from `start` up to the current position, verbatim —
    /// no re-tokenization, no trimming. `ARCHETYPE_HRID`/`ARCHETYPE_REF`
    /// (`archetype_id/value matches`'s own target, `use_archetype`'s second
    /// bracket argument) lex as several `Word`/`Symbol('-')` tokens rather
    /// than one, because this lexer's word-scanner stops at `-`
    /// (`Self::next`'s own comment on `SYMBOLS`) and nothing else in this
    /// parser needs an archetype reference lexed atomically. Slicing the
    /// original source between two offsets a caller has already bounded —
    /// rather than adding a second, `-`-tolerant word-scanning rule that
    /// only this one construct would use — reconstructs the exact text
    /// without guessing at a grammar this parser does not otherwise lex.
    pub(super) fn text_since(&self, start: usize) -> &'a str {
        &self.source[start..self.offset()]
    }

    /// Reads raw, un-tokenized text up to the next whitespace or the end of
    /// input — `ADL_PATH`'s own shape (`base_lexer.g4`): a run of
    /// `/`-separated segments with no unescaped whitespace inside it, so
    /// this is exact, not approximate. `use_node`'s trailing target path is
    /// the only construct this parser reads this way; everything else it
    /// parses is well served by `Self::next`'s own token boundaries.
    pub(super) fn read_raw_path(&mut self) -> Option<&'a str> {
        let text = self.peek_raw_path()?;
        self.rest = &self.rest[text.len()..];
        Some(text)
    }

    /// [`Self::read_raw_path`] without consuming: the same whitespace-bounded
    /// slice, left in place, so `c_attribute` can decide whether the text
    /// ahead is an `ADL_PATH` (it contains `/`) or a bare attribute name
    /// before committing to reading it either way (`A-70`). Leading trivia
    /// *is* consumed, as `Self::peek` consumes it.
    pub(super) fn peek_raw_path(&mut self) -> Option<&'a str> {
        self.skip_trivia();
        if self.rest.is_empty() {
            return None;
        }
        let end = self.rest.find(char::is_whitespace).unwrap_or(self.rest.len());
        Some(&self.rest[..end])
    }

    /// Reads a maximal run of ISO8601 date/time/date-time/duration
    /// characters — digits, `-:+?,`, and the letters `TZPYMWDHS` in either
    /// case (`base_lexer.g4`'s `YEAR`/`MONTH`/`DAY`/`HOUR`/`MINUTE`/
    /// `SECOND`/`TIMEZONE`/`ISO8601_DURATION` fragments, folded into one
    /// character class since every caller re-validates through `T::from_str`
    /// anyway) — plus a `.` only when followed by another digit
    /// (`SECOND_DEC_SEP` allows `.` or `,` before a fractional second), never
    /// when followed by a second `.`: a `..` range separator must stay a
    /// range separator, not the start of one of these literals (`A-65`).
    ///
    /// This is a lexical scan, not a validator: no dedicated ISO8601 token
    /// exists in this lexer, unlike `INTEGER`/`REAL`'s own number-scanning
    /// (`A-65` — before this method, `2024-01-01` lexed as `Integer("2024")`,
    /// `Symbol('-')`, …, five tokens, because the word-scanner treats `-`
    /// as a symbol). [`super::cadl::expect_temporal`] is the only caller,
    /// and only where a temporal literal is grammatically expected, so
    /// there is no ambiguity with a plain `INTEGER`/`REAL` for this scan to
    /// resolve — it hands the result straight to `T::from_str`, the same as
    /// [`Self::next`] already does for a `Word`-shaped token elsewhere.
    pub(super) fn read_iso8601(&mut self) -> Option<&'a str> {
        let text = self.peek_iso8601()?;
        self.rest = &self.rest[text.len()..];
        Some(text)
    }

    /// [`Self::read_iso8601`] without consuming — the same maximal run of
    /// [`is_iso8601_char`], left in place, so a caller can classify it (a
    /// value, a `*_CONSTRAINT_PATTERN`, or neither) before committing to
    /// reading it (`A-75`, mirroring [`Self::peek_raw_path`]).
    pub(super) fn peek_iso8601(&mut self) -> Option<&'a str> {
        self.skip_trivia();
        let mut end = 0;
        let mut chars = self.rest.char_indices().peekable();
        while let Some(&(i, c)) = chars.peek() {
            let is_fraction_dot =
                c == '.' && self.rest[i + 1..].starts_with(|next: char| next.is_ascii_digit());
            if is_iso8601_char(c) || is_fraction_dot {
                end = i + c.len_utf8();
                chars.next();
            } else {
                break;
            }
        }
        if end == 0 {
            return None;
        }
        Some(&self.rest[..end])
    }

    /// Attempts to read a `CONTAINED_REGEXP`'s own delimited pattern —
    /// `'{' WS* (SLASH_REGEXP | CARET_REGEXP)` (`base_lexer.g4`) — leaving
    /// the lexer positioned right after the closing delimiter, *before*
    /// `CONTAINED_REGEXP`'s own optional `';' STRING` assumed value and
    /// mandatory closing `'}'`: both of those are ordinary tokens
    /// (`Symbol`/`Str`), read through [`Self::next`] by the caller, unlike
    /// the regex body itself, which is not validly tokenizable that way —
    /// it may contain `{`, `,`, `"`, anything but an unescaped delimiter or
    /// a newline.
    ///
    /// Returns `Ok(None)`, consuming nothing, when the next non-trivia
    /// text is not shaped like a `CONTAINED_REGEXP` at all — no leading
    /// `'{'`, or a `'{'` whose first non-trivia character is neither `/`
    /// nor `^` — so the caller falls back to its own plain `'{' ... '}'`
    /// handling (`c_objects`, or an `ARCHETYPE_SLOT`'s own `include`/
    /// `exclude` block). No other lexical shape in this grammar starts a
    /// brace-delimited block with `/` or `^`, so this dispatch is exact,
    /// not a guess.
    ///
    /// Returns `Err(())` for a `'{'` that *did* start a regex but has no
    /// matching unescaped closing delimiter before a newline or the end of
    /// input, or whose body is empty (`base_lexer.g4`'s own
    /// `SLASH_REGEXP_CHAR+`/`CARET_REGEXP_CHAR+` both require one or more)
    /// — the caller attributes the offset, since this method does not
    /// track one of its own.
    ///
    /// The returned text includes both delimiters, exactly as written,
    /// backslash escapes and all — the same "carried, not evaluated" form
    /// [`crate::am::CPrimitive::String`]'s own regex `list` elements
    /// already use (`A-63`).
    pub(super) fn try_read_contained_regexp(&mut self) -> Result<Option<&'a str>, ()> {
        self.skip_trivia();
        if !self.rest.starts_with('{') {
            return Ok(None);
        }
        let after_brace = self.rest[1..].trim_start();
        let Some(delimiter @ ('/' | '^')) = after_brace.chars().next() else {
            return Ok(None);
        };
        let body = &after_brace[delimiter.len_utf8()..];
        let mut chars = body.char_indices();
        let mut close = None;
        while let Some((i, c)) = chars.next() {
            if c == '\\' {
                chars.next();
                continue;
            }
            if c == delimiter {
                close = Some(i);
                break;
            }
            if c == '\n' || c == '\r' {
                break;
            }
        }
        let Some(close) = close else {
            return Err(());
        };
        if close == 0 {
            return Err(());
        }
        let end = delimiter.len_utf8() + close + delimiter.len_utf8();
        let text = &after_brace[..end];
        self.rest = &after_brace[end..];
        Ok(Some(text))
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

    /// The first bound of an interval whose opening `|` is the next token,
    /// without consuming anything: the token after `|` and any run of
    /// relational or sign symbols (`>`, `<`, `=`, `+`, `-`), paired with the
    /// raw text from that token onward. `None` if the next token is not `|`
    /// or nothing follows it.
    ///
    /// `A-72`: `odin_values.g4` builds `integer_interval_value` from
    /// `INTEGER` tokens and `real_interval_value` from `REAL` tokens, so the
    /// kind of an unwrapped interval is decided by its first bound's token
    /// — `|0..100|` is a `C_INTEGER`, `|0.0..100.0|` a `C_REAL` — and the
    /// parser asks here before committing to either. The raw text lets it
    /// tell an ISO 8601 bound (`|2024-01-01..2024-12-31|`, which also
    /// begins with digits) from a number.
    pub(super) fn peek_interval_bound(&mut self) -> Option<(Token, &'a str)> {
        let checkpoint = self.rest;
        let mut found = None;
        if matches!(self.next(), Some(Token::Symbol('|'))) {
            loop {
                self.skip_trivia();
                let at = self.rest;
                match self.next() {
                    Some(Token::Symbol('>' | '<' | '=' | '+' | '-')) => {}
                    Some(token) => {
                        found = Some((token, at));
                        break;
                    }
                    None => break,
                }
            }
        }
        self.rest = checkpoint;
        found
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

    #[test]
    fn text_since_reconstructs_an_archetype_ref_split_across_several_tokens() {
        let mut lexer = Lexer::new("openEHR-EHR-CLUSTER.device.v1]");
        let start = lexer.offset();
        while !matches!(lexer.peek(), Some(Token::Symbol(']')) | None) {
            lexer.next();
        }
        assert_eq!(lexer.text_since(start), "openEHR-EHR-CLUSTER.device.v1");
    }

    #[test]
    fn read_raw_path_stops_at_whitespace_and_skips_leading_trivia() {
        let mut lexer = Lexer::new("  -- a comment\n /data[at0001]  matches");
        assert_eq!(lexer.read_raw_path(), Some("/data[at0001]"));
        assert_eq!(lexer.next(), Some(Token::Word("matches".to_owned())));
    }

    #[test]
    fn read_raw_path_returns_none_at_end_of_input() {
        let mut lexer = Lexer::new("   ");
        assert_eq!(lexer.read_raw_path(), None);
    }

    #[test]
    fn peek_raw_path_leaves_the_text_in_place() {
        let mut lexer = Lexer::new("  /data/events cardinality");
        assert_eq!(lexer.peek_raw_path(), Some("/data/events"));
        assert_eq!(lexer.peek_raw_path(), Some("/data/events"));
        assert_eq!(lexer.read_raw_path(), Some("/data/events"));
        assert_eq!(lexer.peek_raw_path(), Some("cardinality"));
    }

    /// `A-65`: before `read_iso8601` existed, none of these lexed as one
    /// token — `2024-01-01` alone split into `Integer("2024")`,
    /// `Symbol('-')`, `Integer("01")`, `Symbol('-')`, `Integer("01")`.
    #[test]
    fn read_iso8601_reads_date_time_date_time_and_duration_literals_whole() {
        for literal in [
            "2024-01-01",
            "12:30:00",
            "12:30:00.500",
            "2024-01-01T12:30:00+0100",
            "2024-01-01T12:30:00Z",
            "P1Y2M3DT4H5M6S",
            "19??-01",
        ] {
            let mut lexer = Lexer::new(literal);
            assert_eq!(lexer.read_iso8601(), Some(literal), "literal: {literal}");
            assert_eq!(lexer.next(), None, "literal: {literal}");
        }
    }

    /// The one boundary `read_iso8601` exists to get right: a `..` range
    /// separator must stay a range separator, never be swallowed as the
    /// start of a fractional-second `.` — the same ambiguity
    /// `a_range_between_two_reals_does_not_swallow_the_dot_dot` proves this
    /// lexer already resolves correctly for `REAL`.
    #[test]
    fn read_iso8601_stops_before_a_dot_dot_range_separator() {
        let mut lexer = Lexer::new("2024-01-01..2024-12-31");
        assert_eq!(lexer.read_iso8601(), Some("2024-01-01"));
        assert_eq!(lexer.next(), Some(Token::DotDot));
        assert_eq!(lexer.read_iso8601(), Some("2024-12-31"));
    }

    /// The other boundary: a `;` (assumed-value separator) or `}` (closing
    /// the enclosing `matches {...}`) must not be swallowed either.
    #[test]
    fn read_iso8601_stops_before_a_semicolon_or_closing_brace() {
        let mut lexer = Lexer::new("2024-01-01; 2024-06-15}");
        assert_eq!(lexer.read_iso8601(), Some("2024-01-01"));
        assert_eq!(lexer.next(), Some(Token::Symbol(';')));
        assert_eq!(lexer.read_iso8601(), Some("2024-06-15"));
        assert_eq!(lexer.next(), Some(Token::Symbol('}')));
    }

    #[test]
    fn read_iso8601_returns_none_at_end_of_input() {
        let mut lexer = Lexer::new("   ");
        assert_eq!(lexer.read_iso8601(), None);
    }

    #[test]
    fn try_read_contained_regexp_reads_slash_and_caret_delimited_forms() {
        let mut lexer = Lexer::new(r"{/foo.*bar/} rest");
        assert_eq!(lexer.try_read_contained_regexp(), Ok(Some("/foo.*bar/")));
        // Positioned right after the closing delimiter, before the `}` —
        // the caller's own job, via the ordinary tokenizer.
        assert_eq!(lexer.next(), Some(Token::Symbol('}')));
        assert_eq!(lexer.next(), Some(Token::Word("rest".to_owned())));

        let mut lexer = Lexer::new(r"{^foo.*bar^}");
        assert_eq!(lexer.try_read_contained_regexp(), Ok(Some("^foo.*bar^")));
    }

    #[test]
    fn try_read_contained_regexp_keeps_an_escaped_delimiter_inside_the_body() {
        let mut lexer = Lexer::new(r"{/mm\[Hg\]|kPa/}");
        assert_eq!(lexer.try_read_contained_regexp(), Ok(Some(r"/mm\[Hg\]|kPa/")));
    }

    /// Not a `CONTAINED_REGEXP` at all — an ordinary `{c_objects}` block, or
    /// a wrapped primitive's `{"literal"}` — nothing is consumed, so the
    /// caller's own `'{' ... '}'` handling still sees the opening brace.
    #[test]
    fn try_read_contained_regexp_returns_none_and_consumes_nothing_for_a_plain_brace() {
        let mut lexer = Lexer::new(r#"{"literal"}"#);
        assert_eq!(lexer.try_read_contained_regexp(), Ok(None));
        assert_eq!(lexer.next(), Some(Token::Symbol('{')));
    }

    #[test]
    fn try_read_contained_regexp_errs_with_no_closing_delimiter() {
        let mut lexer = Lexer::new("{/unterminated");
        assert_eq!(lexer.try_read_contained_regexp(), Err(()));

        // A newline before the closing delimiter is malformed too — a
        // regex body permits neither `\n` nor `\r` (`base_lexer.g4`'s own
        // `SLASH_REGEXP_CHAR`).
        let mut lexer = Lexer::new("{/no\nclose/}");
        assert_eq!(lexer.try_read_contained_regexp(), Err(()));
    }

    #[test]
    fn try_read_contained_regexp_errs_on_an_empty_body() {
        let mut lexer = Lexer::new("{//}");
        assert_eq!(lexer.try_read_contained_regexp(), Err(()));
    }
}
