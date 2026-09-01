//! The tokenizer [`crate::am::adl14`] and [`crate::am::adl2`] both need for
//! their header-only readers, factored out once rather than kept as two
//! near-identical copies — ADL 1.4 and ADL 2 disagree about what a header
//! contains (a `concept` line versus none at all), but agree on what a token
//! looks like: a word, a bracket, or a comment to skip.
//!
//! Deliberately minimal and **not** a general ADL/cADL/ODIN lexer: no string
//! literals, no numbers beyond what a bare word already covers, no `<` `>`
//! value-wrapper recognition. Each of those belongs to a section neither
//! header reader parses.

/// One lexical token: a bare word, or one of ADL's header-level structural
/// characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    /// A run of everything but whitespace and the structural characters
    /// below — covers keywords (`archetype`, `specialize`, `concept`), an
    /// `ARCHETYPE_HRID`/`ARCHETYPE_REF`, and an `AT_CODE`/`ID_CODE`. Which of
    /// those it is depends on context, exactly as the real grammar resolves
    /// its identifier token against keyword tokens before falling back to it.
    Word(String),
    /// `(`, `)`, `[`, `]`, `;`, `=`.
    Symbol(char),
}

impl core::fmt::Display for Token {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Word(w) => write!(f, "`{w}`"),
            Self::Symbol(s) => write!(f, "`{s}`"),
        }
    }
}

/// The structural characters a header-only reader needs to recognise.
const SYMBOLS: &str = "()[];=";

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

    /// The byte offset the lexer has read up to — where the *next* token
    /// would start once trivia is skipped, or where reading stopped for an
    /// error raised right after a [`Self::next`] call.
    pub(super) fn offset(&self) -> usize {
        self.source.len() - self.rest.len()
    }

    /// Skips whitespace and `-- ...` line comments, both insignificant
    /// everywhere between tokens in ADL, not only at line boundaries.
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

    pub(super) fn next(&mut self) -> Option<Token> {
        self.skip_trivia();
        let mut chars = self.rest.char_indices();
        let (_, first) = chars.next()?;
        if SYMBOLS.contains(first) {
            self.rest = &self.rest[first.len_utf8()..];
            return Some(Token::Symbol(first));
        }
        let end = chars
            .find(|&(_, c)| c.is_whitespace() || SYMBOLS.contains(c))
            .map_or(self.rest.len(), |(i, _)| i);
        let word = &self.rest[..end];
        self.rest = &self.rest[end..];
        Some(Token::Word(word.to_owned()))
    }

    /// Peeks at the next token's text without consuming it, for the
    /// optional-keyword lookahead both header readers need (`specialize`,
    /// present or not, before committing to consuming it). `None` when the
    /// next token is a symbol, not a word, or when input is exhausted.
    pub(super) fn peek_word(&mut self) -> Option<&str> {
        self.skip_trivia();
        let first = self.rest.chars().next()?;
        if SYMBOLS.contains(first) {
            return None;
        }
        self.rest
            .split(|c: char| c.is_whitespace() || SYMBOLS.contains(c))
            .next()
            .filter(|w| !w.is_empty())
    }

    /// Whether the next token, after skipping trivia, is the given symbol —
    /// without consuming anything. Used for `meta_data`'s optional leading
    /// `(`, which [`Self::peek_word`] cannot see (it only recognises words).
    pub(super) fn peek_symbol_is(&mut self, want: char) -> bool {
        self.skip_trivia();
        self.rest.starts_with(want)
    }

    /// Whether input is exhausted, after skipping trivia.
    pub(super) fn at_end(&mut self) -> bool {
        self.skip_trivia();
        self.rest.is_empty()
    }

    /// Consumes a balanced `( ... )` block without interpreting its
    /// contents — the header's optional `meta_data`, which neither reader
    /// carries anywhere: [`crate::am::Archetype`]'s own `adl_version` field
    /// exists, but nothing either reader produces is an `Archetype`.
    ///
    /// # Errors
    ///
    /// Returns `Err(offset)` at an unterminated `(...)` — the caller wraps
    /// it in its own error type.
    pub(super) fn skip_parenthesised(&mut self) -> Result<(), usize> {
        debug_assert!(matches!(self.next(), Some(Token::Symbol('('))));
        let mut depth = 1u32;
        loop {
            let offset = self.offset();
            match self.next() {
                Some(Token::Symbol('(')) => depth += 1,
                Some(Token::Symbol(')')) => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                Some(_) => {}
                None => return Err(offset),
            }
        }
    }
}
