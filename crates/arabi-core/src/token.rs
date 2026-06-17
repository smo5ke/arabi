use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    FString(String),
    Boolean(bool),
    Null,

    // Identifiers
    Identifier(String),

    // Keywords
    Keyword(Keyword),

    // Operators
    Operator(Operator),

    // Delimiters
    Delimiter(Delimiter),

    // Special
    Newline,
    Indent(usize),
    Dedent(usize),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

impl SpannedToken {
    pub fn new(token: Token, span: Span) -> Self {
        SpannedToken { token, span }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    // Control flow
    If,
    Elif,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    Pass,

    // Functions
    Function,
    Return,
    Lambda,

    // Classes
    Class,
    Self_,
    Super,

    // Import
    Import,
    From,
    As,

    // Exception handling
    Try,
    Except,
    Finally,
    Raise,

    // Other
    Delete,
    Assert,
    Is,
    Global,
    Nonlocal,
    And,
    Or,
    Not,
    True,
    False,
    None,

    // Generators
    Yield,
    YieldFrom,

    // Context managers
    With,

    // Decorator
    Decorator,

    // Match/Case
    Match,
    Case,
    CaseDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    // Arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    Backslash,
    DoubleStar,
    DoubleBackslash,
    Caret,
    Percent,

    // Comparison
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,

    // Bitwise
    Ampersand,
    Pipe,
    Tilde,
    Shl,
    Shr,

    // Assignment
    Assign,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    BackslashEq,
    DoubleStarEq,
    DoubleBackslashEq,
    CaretEq,
    PercentEq,
    AmpersandEq,
    PipeEq,
    ShlEq,
    ShrEq,
    WalrusEq,

    // Arrow
    Arrow,

    // Decorator
    At,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delimiter {
    Colon,
    Semicolon,
    Comma,
    Dot,
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
}
