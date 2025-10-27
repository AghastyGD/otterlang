use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Keywords
    Fn,
    Let,
    Return,
    If,
    Else,
    Elif,
    For,
    While,
    Break,
    Continue,
    In,
    Use,
    From,
    As,
    Async,
    Await,
    Spawn,
    Match,
    Case,
    True,
    False,
    Print,

    // Identifiers
    Identifier(String),
    UnicodeIdentifier(String),

    // Literals
    Number(String),
    StringLiteral(String),
    FString(String), // Raw f-string content like "π ≈ {result}"
    Bool(bool),

    // Structural
    Colon,
    Newline,
    Indent,
    Dedent,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,

    // Operators
    Arrow,
    Equals,
    EqEq,
    Neq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Pipe,
    Amp,
    Bang,

    // Assignment operators
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,

    // Range operator
    DoubleDot,

    Eof,
}

// FStringPart is now defined in the AST module

impl TokenKind {
    pub fn name(&self) -> &'static str {
        match self {
            // Keywords
            TokenKind::Fn => "fn",
            TokenKind::Let => "let",
            TokenKind::Return => "return",
            TokenKind::If => "if",
            TokenKind::Else => "else",
            TokenKind::Elif => "elif",
            TokenKind::For => "for",
            TokenKind::While => "while",
            TokenKind::Break => "break",
            TokenKind::Continue => "continue",
            TokenKind::In => "in",
            TokenKind::Use => "use",
            TokenKind::From => "from",
            TokenKind::As => "as",
            TokenKind::Async => "async",
            TokenKind::Await => "await",
            TokenKind::Spawn => "spawn",
            TokenKind::Match => "match",
            TokenKind::Case => "case",
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::Print => "print",

            // Identifiers
            TokenKind::Identifier(_) => "identifier",
            TokenKind::UnicodeIdentifier(_) => "unicode_identifier",

            // Literals
            TokenKind::Number(_) => "number",
            TokenKind::StringLiteral(_) => "string",
            TokenKind::FString { .. } => "fstring",
            TokenKind::Bool(_) => "bool",

            // Structural
            TokenKind::Colon => ":",
            TokenKind::Newline => "newline",
            TokenKind::Indent => "indent",
            TokenKind::Dedent => "dedent",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::Comma => ",",
            TokenKind::Dot => ".",

            // Operators
            TokenKind::Arrow => "->",
            TokenKind::Equals => "=",
            TokenKind::EqEq => "==",
            TokenKind::Neq => "!=",
            TokenKind::Lt => "<",
            TokenKind::Gt => ">",
            TokenKind::LtEq => "<=",
            TokenKind::GtEq => ">=",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::Pipe => "|",
            TokenKind::Amp => "&",
            TokenKind::Bang => "!",

            // Assignment operators
            TokenKind::PlusEq => "+=",
            TokenKind::MinusEq => "-=",
            TokenKind::StarEq => "*=",
            TokenKind::SlashEq => "/=",

            // Range operator
            TokenKind::DoubleDot => "..",

            TokenKind::Eof => "eof",
        }
    }
}

impl fmt::Debug for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Identifier(name) => write!(f, "Identifier({name})"),
            TokenKind::UnicodeIdentifier(name) => write!(f, "UnicodeIdentifier({name})"),
            TokenKind::Number(number) => write!(f, "Number({number})"),
            TokenKind::StringLiteral(value) => write!(f, "StringLiteral(\"{value}\")"),
            TokenKind::FString(content) => write!(f, "FString(\"{}\")", content),
            TokenKind::Bool(value) => write!(f, "Bool({value})"),
            kind => f.write_str(kind.name()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
