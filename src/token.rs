use crate::common;

#[macro_export]
macro_rules! Token {
    [,]       => { $crate::token::TokenKind::Comma };
    [.]       => { $crate::token::TokenKind::Dot };
    [-]       => { $crate::token::TokenKind::Minus };
    [+]       => { $crate::token::TokenKind::Plus };
    [;]       => { $crate::token::TokenKind::Semi };
    [/]       => { $crate::token::TokenKind::Slash };
    [*]       => { $crate::token::TokenKind::Star };
    [%]       => { $crate::token::TokenKind::Percent };
    [%]       => { $crate::token::TokenKind::Percent };
    [#]    => { $crate::token::TokenKind::Hash };
    [?]    => { $crate::token::TokenKind::QuestionMark };
    [?.]    => { $crate::token::TokenKind::MarkDot };
    [:]    => { $crate::token::TokenKind::Colon };
    [!]    => { $crate::token::TokenKind::Bang };
    [!=]    => { $crate::token::TokenKind::BangEqual };
    [=]    => { $crate::token::TokenKind::Equal };
    [==]    => { $crate::token::TokenKind::Equal2 };
    [>]    => { $crate::token::TokenKind::Greater };
    [>=]    => { $crate::token::TokenKind::GreaterEqual };
    [->]    => { $crate::token::TokenKind::Arrow };
    [=>]    => { $crate::token::TokenKind::FatArrow };
    [|]    => { $crate::token::TokenKind::Bar };
    [&]    => { $crate::token::TokenKind::Ampersand };
    [||]          => { $crate::token::TokenKind::Bar2 };
    [&&]        => { $crate::token::TokenKind::Ampersand2 };
    [let]        => { $crate::token::TokenKind::Let };
    [true]        => { $crate::token::TokenKind::True };
    [false]        => { $crate::token::TokenKind::False };
    [fn]          => { $crate::token::TokenKind::Fn};
    [if]          => { $crate::token::TokenKind::If };
    [else]          => { $crate::token::TokenKind::Else };
    [for]          => { $crate::token::TokenKind::For };
    [while]          => { $crate::token::TokenKind::While };
    [loop]          => { $crate::token::TokenKind::Loop };
    [continue]          => { $crate::token::TokenKind::Continue };
    [break]          => { $crate::token::TokenKind::Break };
    [in]          => { $crate::token::TokenKind::In };
    [nil]          => { $crate::token::TokenKind::Nil };
    [return]          => { $crate::token::TokenKind::Return };
    [use]          => { $crate::token::TokenKind::Use };
    [struct]          => { $crate::token::TokenKind::Struct };
    [impl]          => { $crate::token::TokenKind::Impl };
    [match]          => { $crate::token::TokenKind::Match };
    [extern]      => { $crate::token::TokenKind::Extern };
    [inline]      => { $crate::token::TokenKind::Inline };
    [self]      => { $crate::token::TokenKind::_self };
    [Self]      => { $crate::token::TokenKind::_Self };
    [EOF]      => { $crate::token::TokenKind::EOF };
    [enum]        => { $crate::token::TokenKind::Enum };
}

#[derive(Clone, Debug, PartialEq)]
pub enum NumberToken {
    Int(i64),
    Float(f64),
}

//TODO: store comments for docs generation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Percent,
    Hash,
    QuestionMark,
    MarkDot,
    Colon,

    Bang,
    BangEqual,
    Equal,
    Equal2,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Arrow,
    FatArrow,
    Bar,
    Ampersand,
    Bar2,
    Ampersand2,

    Identifier,
    Label,
    String,
    Number,

    Let,
    Global,
    True,
    False,
    Fn,
    If,
    Else,
    For,
    While,
    Loop,
    Continue,
    Break,
    In,
    Nil,
    Return,
    Use,
    Struct,
    Impl,
    Match,
    Extern,
    Inline,
    _Self,
    #[allow(non_camel_case_types)]
    _self,

    UnterminatedString,
    Unknown,
    EOF,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Hash => write!(f, "#"),
            TokenKind::MarkDot => write!(f, "?."),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::BangEqual => write!(f, "!="),
            TokenKind::Equal => write!(f, "="),
            TokenKind::Equal2 => write!(f, "=="),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::Bar => write!(f, "|"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Bar2 => write!(f, "||"),
            TokenKind::Ampersand2 => write!(f, "&&"),
            TokenKind::Identifier => write!(f, "identifier"),
            TokenKind::Label => write!(f, "label"),
            TokenKind::String => write!(f, "string"),
            TokenKind::Number => write!(f, "number"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Global => write!(f, "global"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::For => write!(f, "for"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Loop => write!(f, "loop"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Nil => write!(f, "nil"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Use => write!(f, "use"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::_Self => write!(f, "Self"),
            TokenKind::UnterminatedString => write!(f, "unterminated string"),
            TokenKind::Unknown => write!(f, "unknown"),
            TokenKind::EOF => write!(f, "EOF"),
            TokenKind::QuestionMark => write!(f, "?"),
            TokenKind::Extern => write!(f, "extern"),
            TokenKind::Inline => write!(f, "inline"),
            TokenKind::_self => write!(f, "self"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Percent,
    Hash,
    MarkDot,
    QuestionMark,
    Colon,

    Bang,
    BangEqual,
    Equal,
    Equal2,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Arrow,
    FatArrow,
    Bar,
    Ampersand,
    Bar2,
    Ampersand2,

    Identifier(String),
    Label(String),
    String(common::StringKind, String),
    Number(NumberToken),

    Let,
    Global,
    True,
    False,
    Fn,
    If,
    Else,
    For,
    While,
    Loop,
    Continue,
    Break,
    In,
    Nil,
    Return,
    Use,
    Struct,
    Impl,
    Match,
    Extern,
    Inline,
    #[allow(non_camel_case_types)]
    _self,
    _Self,

    Unknown(char),
    EOF,
}

impl Token {
    pub fn kind(&self) -> TokenKind {
        self.into()
    }
}

impl From<&Token> for TokenKind {
    fn from(value: &Token) -> Self {
        match value {
            Token::LeftParen => Self::LeftParen,
            Token::RightParen => Self::RightParen,
            Token::LeftBrace => Self::LeftBrace,
            Token::RightBrace => Self::RightBrace,
            Token::LeftBracket => Self::LeftBracket,
            Token::RightBracket => Self::RightBracket,
            Token::Comma => Self::Comma,
            Token::Dot => Self::Dot,
            Token::Minus => Self::Minus,
            Token::Plus => Self::Plus,
            Token::Semicolon => Self::Semicolon,
            Token::Slash => Self::Slash,
            Token::Star => Self::Star,
            Token::Percent => Self::Percent,
            Token::Hash => Self::Hash,
            Token::MarkDot => Self::MarkDot,
            Token::Colon => Self::Colon,
            Token::Bang => Self::Bang,
            Token::BangEqual => Self::BangEqual,
            Token::Equal => Self::Equal,
            Token::Equal2 => Self::Equal2,
            Token::Greater => Self::Greater,
            Token::GreaterEqual => Self::GreaterEqual,
            Token::Less => Self::Less,
            Token::LessEqual => Self::LessEqual,
            Token::Arrow => Self::Arrow,
            Token::FatArrow => Self::FatArrow,
            Token::Bar => Self::Bar,
            Token::Ampersand => Self::Ampersand,
            Token::Bar2 => Self::Bar2,
            Token::Ampersand2 => Self::Ampersand2,
            Token::Identifier(_) => Self::Identifier,
            Token::Label(_) => Self::Label,
            Token::String(_, _) => Self::String,
            Token::Number(_) => Self::Number,
            Token::Let => Self::Let,
            Token::Global => Self::Global,
            Token::True => Self::True,
            Token::False => Self::False,
            Token::Fn => Self::Fn,
            Token::If => Self::If,
            Token::Else => Self::Else,
            Token::For => Self::For,
            Token::While => Self::While,
            Token::Loop => Self::Loop,
            Token::Continue => Self::Continue,
            Token::Break => Self::Break,
            Token::In => Self::In,
            Token::Nil => Self::Nil,
            Token::Return => Self::Return,
            Token::Use => Self::Use,
            Token::Struct => Self::Struct,
            Token::Impl => Self::Impl,
            Token::Match => Self::Match,
            Token::Unknown(_) => Self::Unknown,
            Token::EOF => Self::EOF,
            Token::QuestionMark => Self::QuestionMark,
            Token::Extern => Self::Extern,
            Token::Inline => Self::Inline,
            Token::_Self => Self::_Self,
            Token::_self => Self::_self,
        }
    }
}
