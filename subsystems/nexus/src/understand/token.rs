//! Token types for lexical analysis
//!
//! This module provides token representation for Rust source code.

extern crate alloc;

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

/// Token identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenId(pub u64);

impl TokenId {
    /// Create new token ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Source location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLoc {
    /// File ID
    pub file_id: u32,
    /// Line number (1-based)
    pub line: u32,
    /// Column number (1-based)
    pub column: u32,
    /// Byte offset
    pub offset: u32,
}

impl SourceLoc {
    /// Create new source location
    pub const fn new(file_id: u32, line: u32, column: u32, offset: u32) -> Self {
        Self {
            file_id,
            line,
            column,
            offset,
        }
    }

    /// Unknown location
    pub const fn unknown() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

/// Source span
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Start location
    pub start: SourceLoc,
    /// End location
    pub end: SourceLoc,
}

impl Span {
    /// Create new span
    pub const fn new(start: SourceLoc, end: SourceLoc) -> Self {
        Self { start, end }
    }

    /// Unknown span
    pub const fn unknown() -> Self {
        Self {
            start: SourceLoc::unknown(),
            end: SourceLoc::unknown(),
        }
    }

    /// Check if spans overlap
    pub fn overlaps(&self, other: &Span) -> bool {
        self.start.offset < other.end.offset && other.start.offset < self.end.offset
    }

    /// Get span length in bytes
    pub fn len(&self) -> u32 {
        self.end.offset.saturating_sub(self.start.offset)
    }

    /// Check if span is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Token kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    /// fn
    Fn,
    /// let
    Let,
    /// mut
    Mut,
    /// const
    Const,
    /// static
    Static,
    /// struct
    Struct,
    /// enum
    Enum,
    /// impl
    Impl,
    /// trait
    Trait,
    /// pub
    Pub,
    /// use
    Use,
    /// mod
    Mod,
    /// if
    If,
    /// else
    Else,
    /// match
    Match,
    /// loop
    Loop,
    /// while
    While,
    /// for
    For,
    /// in
    In,
    /// return
    Return,
    /// break
    Break,
    /// continue
    Continue,
    /// unsafe
    Unsafe,
    /// async
    Async,
    /// await
    Await,
    /// where
    Where,
    /// Self (type)
    SelfType,
    /// self (value)
    SelfValue,
    /// super
    Super,
    /// crate
    Crate,
    /// as
    As,
    /// dyn
    Dyn,
    /// move
    Move,
    /// ref
    Ref,
    /// type
    Type,
    /// extern
    Extern,

    // Literals
    /// Integer literal
    IntLiteral,
    /// Float literal
    FloatLiteral,
    /// String literal
    StringLiteral,
    /// Char literal
    CharLiteral,
    /// Boolean literal
    BoolLiteral,

    // Identifiers
    /// Identifier
    Ident,
    /// Lifetime
    Lifetime,

    // Operators
    /// +
    Plus,
    /// -
    Minus,
    /// *
    Star,
    /// /
    Slash,
    /// %
    Percent,
    /// &
    Ampersand,
    /// |
    Pipe,
    /// ^
    Caret,
    /// !
    Bang,
    /// =
    Eq,
    /// ==
    EqEq,
    /// !=
    Ne,
    /// <
    Lt,
    /// <=
    Le,
    /// >
    Gt,
    /// >=
    Ge,
    /// &&
    AndAnd,
    /// ||
    OrOr,
    /// <<
    Shl,
    /// >>
    Shr,
    /// +=
    PlusEq,
    /// -=
    MinusEq,
    /// *=
    StarEq,
    /// /=
    SlashEq,
    /// %=
    PercentEq,
    /// &=
    AndEq,
    /// |=
    OrEq,
    /// ^=
    CaretEq,
    /// <<=
    ShlEq,
    /// >>=
    ShrEq,

    // Delimiters
    /// (
    OpenParen,
    /// )
    CloseParen,
    /// {
    OpenBrace,
    /// }
    CloseBrace,
    /// [
    OpenBracket,
    /// ]
    CloseBracket,

    // Punctuation
    /// ;
    Semi,
    /// ,
    Comma,
    /// .
    Dot,
    /// ..
    DotDot,
    /// ...
    DotDotDot,
    /// ..=
    DotDotEq,
    /// :
    Colon,
    /// ::
    PathSep,
    /// ->
    Arrow,
    /// =>
    FatArrow,
    /// ?
    Question,
    /// @
    At,
    /// #
    Hash,
    /// $
    Dollar,

    // Special
    /// Whitespace
    Whitespace,
    /// Comment
    Comment,
    /// Doc comment
    DocComment,
    /// End of file
    Eof,
    /// Unknown token
    Unknown,
}

impl TokenKind {
    /// Check if this is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Self::Fn
                | Self::Let
                | Self::Mut
                | Self::Const
                | Self::Static
                | Self::Struct
                | Self::Enum
                | Self::Impl
                | Self::Trait
                | Self::Pub
                | Self::Use
                | Self::Mod
                | Self::If
                | Self::Else
                | Self::Match
                | Self::Loop
                | Self::While
                | Self::For
                | Self::In
                | Self::Return
                | Self::Break
                | Self::Continue
                | Self::Unsafe
                | Self::Async
                | Self::Await
                | Self::Where
                | Self::SelfType
                | Self::SelfValue
                | Self::Super
                | Self::Crate
                | Self::As
                | Self::Dyn
                | Self::Move
                | Self::Ref
                | Self::Type
                | Self::Extern
        )
    }

    /// Check if this is a literal
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            Self::IntLiteral
                | Self::FloatLiteral
                | Self::StringLiteral
                | Self::CharLiteral
                | Self::BoolLiteral
        )
    }

    /// Check if this is an operator
    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            Self::Plus
                | Self::Minus
                | Self::Star
                | Self::Slash
                | Self::Percent
                | Self::Ampersand
                | Self::Pipe
                | Self::Caret
                | Self::Bang
                | Self::Eq
                | Self::EqEq
                | Self::Ne
                | Self::Lt
                | Self::Le
                | Self::Gt
                | Self::Ge
                | Self::AndAnd
                | Self::OrOr
                | Self::Shl
                | Self::Shr
                | Self::PlusEq
                | Self::MinusEq
                | Self::StarEq
                | Self::SlashEq
                | Self::PercentEq
                | Self::AndEq
                | Self::OrEq
                | Self::CaretEq
                | Self::ShlEq
                | Self::ShrEq
        )
    }

    /// Check if this is a delimiter
    pub fn is_delimiter(&self) -> bool {
        matches!(
            self,
            Self::OpenParen
                | Self::CloseParen
                | Self::OpenBrace
                | Self::CloseBrace
                | Self::OpenBracket
                | Self::CloseBracket
        )
    }

    /// Check if this is punctuation
    pub fn is_punctuation(&self) -> bool {
        matches!(
            self,
            Self::Semi
                | Self::Comma
                | Self::Dot
                | Self::DotDot
                | Self::DotDotDot
                | Self::DotDotEq
                | Self::Colon
                | Self::PathSep
                | Self::Arrow
                | Self::FatArrow
                | Self::Question
                | Self::At
                | Self::Hash
                | Self::Dollar
        )
    }
}

/// Token
#[derive(Debug, Clone)]
pub struct Token {
    /// Token ID
    pub id: TokenId,
    /// Token kind
    pub kind: TokenKind,
    /// Token text
    pub text: String,
    /// Token span
    pub span: Span,
}

impl Token {
    /// Create new token
    pub fn new(id: TokenId, kind: TokenKind, text: String, span: Span) -> Self {
        Self {
            id,
            kind,
            text,
            span,
        }
    }

    /// Check if token is trivia (whitespace or comment)
    pub fn is_trivia(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Whitespace | TokenKind::Comment | TokenKind::DocComment
        )
    }

    /// Check if token is EOF
    pub fn is_eof(&self) -> bool {
        self.kind == TokenKind::Eof
    }
}
