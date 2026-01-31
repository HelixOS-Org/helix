//! Lexer for Rust kernel code
//!
//! This module provides lexical analysis for Rust source code.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::token::{SourceLoc, Span, Token, TokenId, TokenKind};

/// Lexer for Rust kernel code
pub struct Lexer {
    /// Source code as bytes
    source: Vec<u8>,
    /// Current position
    position: usize,
    /// Current line
    line: u32,
    /// Current column
    column: u32,
    /// File ID
    file_id: u32,
    /// Token counter
    token_counter: AtomicU64,
}

impl Lexer {
    /// Create new lexer
    pub fn new(source: String, file_id: u32) -> Self {
        Self {
            source: source.into_bytes(),
            position: 0,
            line: 1,
            column: 1,
            file_id,
            token_counter: AtomicU64::new(0),
        }
    }

    /// Get current location
    fn current_loc(&self) -> SourceLoc {
        SourceLoc::new(self.file_id, self.line, self.column, self.position as u32)
    }

    /// Peek current character
    fn peek(&self) -> Option<u8> {
        self.source.get(self.position).copied()
    }

    /// Peek ahead
    fn peek_ahead(&self, n: usize) -> Option<u8> {
        self.source.get(self.position + n).copied()
    }

    /// Advance one character
    fn advance(&mut self) -> Option<u8> {
        let ch = self.peek()?;
        self.position += 1;
        if ch == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    /// Skip whitespace
    fn skip_whitespace(&mut self) -> Option<Token> {
        let start = self.current_loc();
        let mut text = Vec::new();

        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                text.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        if text.is_empty() {
            return None;
        }

        let end = self.current_loc();
        let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));
        Some(Token::new(
            id,
            TokenKind::Whitespace,
            String::from_utf8_lossy(&text).into_owned(),
            Span::new(start, end),
        ))
    }

    /// Skip comment
    fn skip_comment(&mut self) -> Option<Token> {
        let start = self.current_loc();

        if self.peek() != Some(b'/') {
            return None;
        }

        let second = self.peek_ahead(1)?;
        let mut text = Vec::new();

        match second {
            b'/' => {
                // Line comment
                while let Some(ch) = self.advance() {
                    text.push(ch);
                    if ch == b'\n' {
                        break;
                    }
                }
            }
            b'*' => {
                // Block comment
                self.advance(); // /
                self.advance(); // *
                text.extend_from_slice(b"/*");

                let mut depth = 1;
                while depth > 0 {
                    match self.advance() {
                        Some(b'*') if self.peek() == Some(b'/') => {
                            self.advance();
                            text.extend_from_slice(b"*/");
                            depth -= 1;
                        }
                        Some(b'/') if self.peek() == Some(b'*') => {
                            self.advance();
                            text.extend_from_slice(b"/*");
                            depth += 1;
                        }
                        Some(ch) => text.push(ch),
                        None => break,
                    }
                }
            }
            _ => return None,
        }

        let end = self.current_loc();
        let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));

        let kind = if text.starts_with(b"///") || text.starts_with(b"//!") {
            TokenKind::DocComment
        } else {
            TokenKind::Comment
        };

        Some(Token::new(
            id,
            kind,
            String::from_utf8_lossy(&text).into_owned(),
            Span::new(start, end),
        ))
    }

    /// Lex identifier or keyword
    fn lex_ident(&mut self) -> Option<Token> {
        let start = self.current_loc();

        let first = self.peek()?;
        if !first.is_ascii_alphabetic() && first != b'_' {
            return None;
        }

        let mut text = Vec::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'_' {
                text.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        let end = self.current_loc();
        let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));
        let text_str = String::from_utf8_lossy(&text).into_owned();

        let kind = match text_str.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "const" => TokenKind::Const,
            "static" => TokenKind::Static,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "impl" => TokenKind::Impl,
            "trait" => TokenKind::Trait,
            "pub" => TokenKind::Pub,
            "use" => TokenKind::Use,
            "mod" => TokenKind::Mod,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "loop" => TokenKind::Loop,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "unsafe" => TokenKind::Unsafe,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "where" => TokenKind::Where,
            "Self" => TokenKind::SelfType,
            "self" => TokenKind::SelfValue,
            "super" => TokenKind::Super,
            "crate" => TokenKind::Crate,
            "as" => TokenKind::As,
            "dyn" => TokenKind::Dyn,
            "move" => TokenKind::Move,
            "ref" => TokenKind::Ref,
            "type" => TokenKind::Type,
            "extern" => TokenKind::Extern,
            "true" | "false" => TokenKind::BoolLiteral,
            _ => TokenKind::Ident,
        };

        Some(Token::new(id, kind, text_str, Span::new(start, end)))
    }

    /// Lex number
    fn lex_number(&mut self) -> Option<Token> {
        let start = self.current_loc();

        let first = self.peek()?;
        if !first.is_ascii_digit() {
            return None;
        }

        let mut text = Vec::new();
        let mut is_float = false;

        // Check for hex, octal, binary
        if first == b'0' {
            text.push(self.advance().unwrap());
            match self.peek() {
                Some(b'x') | Some(b'X') => {
                    text.push(self.advance().unwrap());
                    while let Some(ch) = self.peek() {
                        if ch.is_ascii_hexdigit() || ch == b'_' {
                            text.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                Some(b'o') | Some(b'O') => {
                    text.push(self.advance().unwrap());
                    while let Some(ch) = self.peek() {
                        if (b'0'..=b'7').contains(&ch) || ch == b'_' {
                            text.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                Some(b'b') | Some(b'B') => {
                    text.push(self.advance().unwrap());
                    while let Some(ch) = self.peek() {
                        if ch == b'0' || ch == b'1' || ch == b'_' {
                            text.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        // Decimal part
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == b'_' {
                text.push(self.advance().unwrap());
            } else if ch == b'.' && self.peek_ahead(1).map_or(false, |c| c.is_ascii_digit()) {
                is_float = true;
                text.push(self.advance().unwrap());
            } else if ch == b'e' || ch == b'E' {
                is_float = true;
                text.push(self.advance().unwrap());
                if let Some(sign) = self.peek() {
                    if sign == b'+' || sign == b'-' {
                        text.push(self.advance().unwrap());
                    }
                }
            } else {
                break;
            }
        }

        // Type suffix
        let suffix_start = text.len();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphabetic() || ch == b'_' {
                text.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        if suffix_start < text.len() {
            let suffix = &text[suffix_start..];
            if suffix.starts_with(b"f32") || suffix.starts_with(b"f64") {
                is_float = true;
            }
        }

        let end = self.current_loc();
        let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));
        let kind = if is_float {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntLiteral
        };

        Some(Token::new(
            id,
            kind,
            String::from_utf8_lossy(&text).into_owned(),
            Span::new(start, end),
        ))
    }

    /// Lex string literal
    fn lex_string(&mut self) -> Option<Token> {
        let start = self.current_loc();

        if self.peek() != Some(b'"') {
            return None;
        }

        let mut text = Vec::new();
        text.push(self.advance().unwrap()); // Opening "

        loop {
            match self.advance() {
                Some(b'\\') => {
                    text.push(b'\\');
                    if let Some(escaped) = self.advance() {
                        text.push(escaped);
                    }
                }
                Some(b'"') => {
                    text.push(b'"');
                    break;
                }
                Some(ch) => text.push(ch),
                None => break,
            }
        }

        let end = self.current_loc();
        let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));

        Some(Token::new(
            id,
            TokenKind::StringLiteral,
            String::from_utf8_lossy(&text).into_owned(),
            Span::new(start, end),
        ))
    }

    /// Lex character literal
    fn lex_char(&mut self) -> Option<Token> {
        let start = self.current_loc();

        if self.peek() != Some(b'\'') {
            return None;
        }

        let mut text = Vec::new();
        text.push(self.advance().unwrap()); // Opening '

        // Check if it's a lifetime
        if let Some(ch) = self.peek() {
            if ch.is_ascii_alphabetic() || ch == b'_' {
                // Could be lifetime or char
                text.push(self.advance().unwrap());

                while let Some(ch) = self.peek() {
                    if ch.is_ascii_alphanumeric() || ch == b'_' {
                        text.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }

                // If no closing quote, it's a lifetime
                if self.peek() != Some(b'\'') {
                    let end = self.current_loc();
                    let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));
                    return Some(Token::new(
                        id,
                        TokenKind::Lifetime,
                        String::from_utf8_lossy(&text).into_owned(),
                        Span::new(start, end),
                    ));
                }
            }
        }

        // Character literal
        loop {
            match self.advance() {
                Some(b'\\') => {
                    text.push(b'\\');
                    if let Some(escaped) = self.advance() {
                        text.push(escaped);
                    }
                }
                Some(b'\'') => {
                    text.push(b'\'');
                    break;
                }
                Some(ch) => text.push(ch),
                None => break,
            }
        }

        let end = self.current_loc();
        let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));

        Some(Token::new(
            id,
            TokenKind::CharLiteral,
            String::from_utf8_lossy(&text).into_owned(),
            Span::new(start, end),
        ))
    }

    /// Lex operator or punctuation
    fn lex_operator(&mut self) -> Option<Token> {
        let start = self.current_loc();
        let first = self.peek()?;

        let (kind, len) = match first {
            b'+' => match self.peek_ahead(1) {
                Some(b'=') => (TokenKind::PlusEq, 2),
                _ => (TokenKind::Plus, 1),
            },
            b'-' => match self.peek_ahead(1) {
                Some(b'=') => (TokenKind::MinusEq, 2),
                Some(b'>') => (TokenKind::Arrow, 2),
                _ => (TokenKind::Minus, 1),
            },
            b'*' => match self.peek_ahead(1) {
                Some(b'=') => (TokenKind::StarEq, 2),
                _ => (TokenKind::Star, 1),
            },
            b'/' => match self.peek_ahead(1) {
                Some(b'=') => (TokenKind::SlashEq, 2),
                Some(b'/') | Some(b'*') => return None, // Comment, handled elsewhere
                _ => (TokenKind::Slash, 1),
            },
            b'%' => match self.peek_ahead(1) {
                Some(b'=') => (TokenKind::PercentEq, 2),
                _ => (TokenKind::Percent, 1),
            },
            b'&' => match self.peek_ahead(1) {
                Some(b'&') => (TokenKind::AndAnd, 2),
                Some(b'=') => (TokenKind::AndEq, 2),
                _ => (TokenKind::Ampersand, 1),
            },
            b'|' => match self.peek_ahead(1) {
                Some(b'|') => (TokenKind::OrOr, 2),
                Some(b'=') => (TokenKind::OrEq, 2),
                _ => (TokenKind::Pipe, 1),
            },
            b'^' => match self.peek_ahead(1) {
                Some(b'=') => (TokenKind::CaretEq, 2),
                _ => (TokenKind::Caret, 1),
            },
            b'!' => match self.peek_ahead(1) {
                Some(b'=') => (TokenKind::Ne, 2),
                _ => (TokenKind::Bang, 1),
            },
            b'=' => match self.peek_ahead(1) {
                Some(b'=') => (TokenKind::EqEq, 2),
                Some(b'>') => (TokenKind::FatArrow, 2),
                _ => (TokenKind::Eq, 1),
            },
            b'<' => match self.peek_ahead(1) {
                Some(b'<') => match self.peek_ahead(2) {
                    Some(b'=') => (TokenKind::ShlEq, 3),
                    _ => (TokenKind::Shl, 2),
                },
                Some(b'=') => (TokenKind::Le, 2),
                _ => (TokenKind::Lt, 1),
            },
            b'>' => match self.peek_ahead(1) {
                Some(b'>') => match self.peek_ahead(2) {
                    Some(b'=') => (TokenKind::ShrEq, 3),
                    _ => (TokenKind::Shr, 2),
                },
                Some(b'=') => (TokenKind::Ge, 2),
                _ => (TokenKind::Gt, 1),
            },
            b':' => match self.peek_ahead(1) {
                Some(b':') => (TokenKind::PathSep, 2),
                _ => (TokenKind::Colon, 1),
            },
            b'.' => match self.peek_ahead(1) {
                Some(b'.') => match self.peek_ahead(2) {
                    Some(b'=') => (TokenKind::DotDotEq, 3),
                    Some(b'.') => (TokenKind::DotDotDot, 3),
                    _ => (TokenKind::DotDot, 2),
                },
                _ => (TokenKind::Dot, 1),
            },
            b'(' => (TokenKind::OpenParen, 1),
            b')' => (TokenKind::CloseParen, 1),
            b'{' => (TokenKind::OpenBrace, 1),
            b'}' => (TokenKind::CloseBrace, 1),
            b'[' => (TokenKind::OpenBracket, 1),
            b']' => (TokenKind::CloseBracket, 1),
            b';' => (TokenKind::Semi, 1),
            b',' => (TokenKind::Comma, 1),
            b'?' => (TokenKind::Question, 1),
            b'@' => (TokenKind::At, 1),
            b'#' => (TokenKind::Hash, 1),
            b'$' => (TokenKind::Dollar, 1),
            _ => return None,
        };

        let mut text = Vec::with_capacity(len);
        for _ in 0..len {
            if let Some(ch) = self.advance() {
                text.push(ch);
            }
        }

        let end = self.current_loc();
        let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));

        Some(Token::new(
            id,
            kind,
            String::from_utf8_lossy(&text).into_owned(),
            Span::new(start, end),
        ))
    }

    /// Get next token
    pub fn next_token(&mut self) -> Token {
        // Skip whitespace and comments
        loop {
            if let Some(token) = self.skip_whitespace() {
                return token;
            }
            if let Some(token) = self.skip_comment() {
                return token;
            }
            break;
        }

        // Try each lexer
        if let Some(token) = self.lex_ident() {
            return token;
        }
        if let Some(token) = self.lex_number() {
            return token;
        }
        if let Some(token) = self.lex_string() {
            return token;
        }
        if let Some(token) = self.lex_char() {
            return token;
        }
        if let Some(token) = self.lex_operator() {
            return token;
        }

        // EOF or unknown
        let start = self.current_loc();
        if self.position >= self.source.len() {
            let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));
            return Token::new(id, TokenKind::Eof, String::new(), Span::new(start, start));
        }

        // Unknown character
        let ch = self.advance().unwrap();
        let end = self.current_loc();
        let id = TokenId(self.token_counter.fetch_add(1, Ordering::Relaxed));
        Token::new(
            id,
            TokenKind::Unknown,
            String::from_utf8_lossy(&[ch]).into_owned(),
            Span::new(start, end),
        )
    }

    /// Tokenize all
    pub fn tokenize_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }
}
