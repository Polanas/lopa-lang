#[derive(Clone, Debug, PartialEq)]
pub enum NumberToken {
    Int(i64),
    Float(f64),
}
//TODO: store comments for docs generation
#[derive(Clone, Debug, PartialEq)]
pub enum TokenVariant {
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
    String(String),
    Number(NumberToken),

    Let,
    Var,
    Global,
    True,
    False,
    Fn,
    If,
    Else,
    For,
    While,
    Loop,
    In,
    Nil,
    Print,
    Return,
    Super,
    Use,
    Struct,
    Impl,
    Match,
    _Self,

    UnterminatedString(String),
    Unknown(char),
}

#[derive(Clone, Debug)]
pub struct Token {
    variant: TokenVariant,
}
