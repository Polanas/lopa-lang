use crate::common;

#[macro_export]
macro_rules! Token {
    [,]       => { $crate::token::TokenKind::Comma };
    [.]       => { $crate::token::TokenKind::Dot };
    [...]       => { $crate::token::TokenKind::Dot3 };
    [-]       => { $crate::token::TokenKind::Minus };
    [-=]       => { $crate::token::TokenKind::MinusEq };
    [+]       => { $crate::token::TokenKind::Plus };
    [+=]       => { $crate::token::TokenKind::PlusEq };
    [;]       => { $crate::token::TokenKind::Semicolon };
    [/]       => { $crate::token::TokenKind::Slash };
    [/=]       => { $crate::token::TokenKind::SlashEq };
    [*]       => { $crate::token::TokenKind::Star };
    [*=]       => { $crate::token::TokenKind::StarEq };
    [%]       => { $crate::token::TokenKind::Percent };
    [%=]       => { $crate::token::TokenKind::PercentEq };
    [#]    => { $crate::token::TokenKind::Hash };
    [?]    => { $crate::token::TokenKind::Mark };
    [??]    => { $crate::token::TokenKind::Mark2 };
    [?.]    => { $crate::token::TokenKind::MarkDot };
    [:]    => { $crate::token::TokenKind::Colon };
    [::]    => { $crate::token::TokenKind::Colon2 };
    [!]    => { $crate::token::TokenKind::Bang };
    [!=]    => { $crate::token::TokenKind::BangEq };
    [=]    => { $crate::token::TokenKind::Eq };
    [==]    => { $crate::token::TokenKind::Eq2 };
    [>]    => { $crate::token::TokenKind::Greater };
    [>>]    => { $crate::token::TokenKind::Greater2 };
    [>>=]    => { $crate::token::TokenKind::Greater2Eq };
    [<]    => { $crate::token::TokenKind::Less };
    [<<]    => { $crate::token::TokenKind::Less2 };
    [<<=]    => { $crate::token::TokenKind::Less2Eq };
    [<=]    => { $crate::token::TokenKind::LessEq };
    [>=]    => { $crate::token::TokenKind::GreaterEq };
    [->]    => { $crate::token::TokenKind::Arrow };
    [=>]    => { $crate::token::TokenKind::FatArrow };
    [|]    => { $crate::token::TokenKind::Bar };
    [&]    => { $crate::token::TokenKind::Ampersand };
    [^]    => { $crate::token::TokenKind::Caret };
    [and]          => { $crate::token::TokenKind::And };
    [or]        => { $crate::token::TokenKind::Or };
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
    [self]      => { $crate::token::TokenKind::SelfValue };
    [Self]      => { $crate::token::TokenKind::SelfType };
    [EOF]      => { $crate::token::TokenKind::EOF };
    [enum]        => { $crate::token::TokenKind::Enum };
}

#[derive(Clone, Debug, PartialEq)]
pub enum NumberToken {
    Int(i64),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StringToken {
    pub value: String,
    pub kind: common::StringKind,
    pub interpolated: bool,
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
    Dot3,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Slash2,
    Slash2Eq,
    Star,
    Percent,
    Hash,
    Mark,
    MarkDot,
    Mark2,
    Colon,
    Colon2,
    Caret,
    MinusEq,
    PlusEq,
    SlashEq,
    StarEq,
    PercentEq,
    CaretEq,
    AmpersandEq,
    BarEq,
    Less2Eq,
    Greater2Eq,

    Bang,
    BangEq,
    Eq,
    Eq2,
    Greater2,
    GreaterEq,
    Greater,
    Less,
    Less2,
    LessEq,
    Arrow,
    FatArrow,
    Bar,
    Ampersand,

    Ident,
    Label,
    String,
    Number,

    Dollar,
    And,
    Or,
    Let,
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
    SelfType,
    #[allow(non_camel_case_types)]
    SelfValue,

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
            TokenKind::Colon2 => write!(f, "::"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::BangEq => write!(f, "!="),
            TokenKind::Eq => write!(f, "="),
            TokenKind::Eq2 => write!(f, "=="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEq => write!(f, "<="),
            TokenKind::Less2Eq => write!(f, "<<="),
            TokenKind::Less2 => write!(f, "<<"),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEq => write!(f, ">="),
            TokenKind::Greater2Eq => write!(f, ">>="),
            TokenKind::Greater2 => write!(f, ">>"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::Bar => write!(f, "|"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Ident => write!(f, "identifier"),
            TokenKind::Label => write!(f, "label"),
            TokenKind::String => write!(f, "string"),
            TokenKind::Number => write!(f, "number"),
            TokenKind::Let => write!(f, "let"),
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
            TokenKind::SelfType => write!(f, "Self"),
            TokenKind::UnterminatedString => write!(f, "unterminated string"),
            TokenKind::Unknown => write!(f, "unknown"),
            TokenKind::EOF => write!(f, "EOF"),
            TokenKind::Mark => write!(f, "?"),
            TokenKind::Extern => write!(f, "extern"),
            TokenKind::Inline => write!(f, "inline"),
            TokenKind::SelfValue => write!(f, "self"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::MinusEq => write!(f, "-="),
            TokenKind::PlusEq => write!(f, "+="),
            TokenKind::SlashEq => write!(f, "/="),
            TokenKind::StarEq => write!(f, "*="),
            TokenKind::PercentEq => write!(f, "%="),
            TokenKind::CaretEq => write!(f, "^="),
            TokenKind::AmpersandEq => write!(f, "&="),
            TokenKind::BarEq => write!(f, "|="),
            TokenKind::Dot3 => write!(f, "..."),
            TokenKind::Mark2 => write!(f, "??"),
            TokenKind::Slash2 => write!(f, "//"),
            TokenKind::Slash2Eq => write!(f, "//="),
            TokenKind::Dollar => write!(f, "$"),
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
    Dot3,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Slash2,
    Slash2Eq,
    Star,
    Percent,
    Hash,
    MarkDot,
    Mark,
    Mark2,
    Colon,
    Colon2,
    Caret,
    MinusEq,
    PlusEq,
    SlashEq,
    StarEq,
    PercentEq,
    CaretEq,
    AmpersandEq,
    BarEq,
    Less2Eq,
    Greater2Eq,
    Less2,
    Greater2,

    Bang,
    BangEq,
    Eq,
    Eq2,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Arrow,
    FatArrow,
    Bar,
    Ampersand,

    Ident(String),
    Label(String),
    String(StringToken),
    Number(NumberToken),

    Dollar,
    And,
    Or,
    Let,
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
    SelfValue,
    SelfType,

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
            Token::Colon2 => Self::Colon2,
            Token::Bang => Self::Bang,
            Token::BangEq => Self::BangEq,
            Token::Eq => Self::Eq,
            Token::Eq2 => Self::Eq2,
            Token::Greater => Self::Greater2,
            Token::GreaterEq => Self::GreaterEq,
            Token::Less => Self::Less,
            Token::LessEq => Self::LessEq,
            Token::Arrow => Self::Arrow,
            Token::FatArrow => Self::FatArrow,
            Token::Bar => Self::Bar,
            Token::Ampersand => Self::Ampersand,
            Token::Or => Self::Or,
            Token::And => Self::And,
            Token::Ident(_) => Self::Ident,
            Token::Label(_) => Self::Label,
            Token::String(_) => Self::String,
            Token::Number(_) => Self::Number,
            Token::Let => Self::Let,
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
            Token::Mark => Self::Mark,
            Token::Extern => Self::Extern,
            Token::Inline => Self::Inline,
            Token::SelfType => Self::SelfType,
            Token::SelfValue => Self::SelfValue,
            Token::Caret => Self::Caret,
            Token::MinusEq => Self::MinusEq,
            Token::PlusEq => Self::PlusEq,
            Token::SlashEq => Self::SlashEq,
            Token::StarEq => Self::StarEq,
            Token::PercentEq => Self::PercentEq,
            Token::CaretEq => Self::CaretEq,
            Token::AmpersandEq => Self::AmpersandEq,
            Token::BarEq => Self::BarEq,
            Token::Less2Eq => Self::Less2Eq,
            Token::Greater2Eq => Self::Greater2Eq,
            Token::Less2 => Self::Less2,
            Token::Greater2 => Self::Greater2,
            Token::Dot3 => Self::Dot3,
            Token::Mark2 => Self::Mark2,
            Token::Slash2 => Self::Slash2,
            Token::Slash2Eq => Self::Slash2Eq,
            Token::Dollar => Self::Dollar,
        }
    }
}
