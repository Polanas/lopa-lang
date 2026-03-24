use logos::Logos;

#[macro_export]
macro_rules! T {
    ['('] => { $crate::lexer::Syntax::LeftParen };
    [')'] => { $crate::lexer::Syntax::RightParen };
    ['{'] => { $crate::lexer::Syntax::LeftBrace };
    ['}'] => { $crate::lexer::Syntax::RightBrace };
    ['['] => { $crate::lexer::Syntax::LeftBracket };
    [']'] => { $crate::lexer::Syntax::RightBracket };
    [' '] => { $crate::lexer::Syntax::Whitespaces };
    [|] => { $crate::lexer::Syntax::Bar };
    [=] => { $crate::lexer::Syntax::Eq };
    [,] => { $crate::lexer::Syntax::Comma };
    [;] => { $crate::lexer::Syntax::Semi };
    [:] => { $crate::lexer::Syntax::Colon };
    [.] => { $crate::lexer::Syntax::Dot };
    [+] => { $crate::lexer::Syntax::Plus };
    [-] => { $crate::lexer::Syntax::Minus };
    [/] => { $crate::lexer::Syntax::Slash };
    [*] => { $crate::lexer::Syntax::Star };
    [->] => { $crate::lexer::Syntax::Arrow };
    [fn] => { $crate::lexer::Syntax::Fn_KW };
    [let] => { $crate::lexer::Syntax::Let_KW };
    [ident] => { $crate::lexer::Syntax::Ident };
    [true] => { $crate::lexer::Syntax::False_KW };
    [false] => { $crate::lexer::Syntax::True_KW };
    [int] => { $crate::lexer::Syntax::Int };
    [float] => { $crate::lexer::Syntax::Float };
    [eof] => { $crate::lexer::Syntax::EndOfFile };
}

#[allow(non_camel_case_types)]
#[derive(Logos, Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash, strum::Display)]
#[repr(u16)]
pub enum Syntax {
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("|")]
    Bar,
    #[token("=")]
    Eq,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("/")]
    Slash,
    #[token("*")]
    Star,
    #[token(";")]
    Semi,
    #[token(":")]
    Colon,
    #[token("let")]
    Let_KW,
    #[token("true")]
    True_KW,
    #[token("false")]
    False_KW,
    #[token("fn")]
    Fn_KW,
    #[token("->")]
    Arrow,
    #[regex(r"[_]?[A-Za-z_][0-9A-Za-z_]*")]
    Ident,
    #[regex(r"[\d][\d|_]*")]
    Int,
    #[regex(r"[\d][\d|_]*\.[\d]+")]
    Float,
    #[regex(r"\s+")]
    Whitespaces,
    Error,
    #[end]
    EndOfFile,
    File,

    FnItem,

    Arg,
    ArgList,
    ParamList,
    Param,
    ReturnType,

    LetStmt,
    ExprStmt,

    TypeExpr,
    LiteralExpr,
    ParenExpr,
    AssignExpr,
    CallExpr,
    IndexExpr,
    ReturnExpr,
    BlockExpr,
    BinaryExpr,
}

impl Syntax {
    // pub fn to_string(&self) -> &str {
    //     match self {
    //         Syntax::LeftParen => "(",
    //         Syntax::RightParen => ")",
    //         Syntax::LeftBracket => "[",
    //         Syntax::RightBracket => "]",
    //         Syntax::LeftBrace => "{",
    //         Syntax::RightBrace => "}",
    //         Syntax::Bar => "|",
    //         Syntax::Eq => "=",
    //         Syntax::Comma => ",",
    //         Syntax::Dot => ".",
    //         Syntax::Plus => "+",
    //         Syntax::Minus => "-",
    //         Syntax::Slash => "/",
    //         Syntax::Star => "*",
    //         Syntax::Semi => ";",
    //         Syntax::Let_KW => "let",
    //         Syntax::Fn_KW => "fn",
    //         Syntax::Arrow => "->",
    //         Syntax::Ident => "ident",
    //         Syntax::Int => "num",
    //         Syntax::Whitespaces => " ",
    //         Syntax::True_KW => "true",
    //         Syntax::False_KW => "false",
    //         Syntax::Error
    //         | Syntax::EndOfFile
    //         | Syntax::File
    //         | Syntax::FnItem
    //         | Syntax::LetStmt
    //         | Syntax::AssignExpr
    //         | Syntax::CallExpr
    //         | Syntax::ReturnExpr
    //         | Syntax::BlockExpr
    //         | Syntax::ExprStmt => panic!("no text"),
    //         Syntax::Float => todo!(),
    //         Syntax::LiteralExpr => todo!(),
    //         Syntax::Arg => todo!(),
    //         Syntax::ArgList => todo!(),
    //         Syntax::ParenExpr => todo!(),
    //     }
    // }
}
impl From<Syntax> for rowan::SyntaxKind {
    fn from(value: Syntax) -> Self {
        Self(value as u16)
    }
}

#[cfg(test)]
mod test {
    use super::Syntax;
    use itertools::Itertools;
    use logos::Logos as _;

    #[test]
    fn simple_fn() {
        let lex = Syntax::lexer("fn main() { let x = 5; }")
            .map(|t| t.unwrap())
            .filter(|t| *t != Syntax::Whitespaces)
            .collect_vec();

        assert_eq!(
            lex.as_slice(),
            &[
                Syntax::Fn_KW,
                Syntax::Ident,
                Syntax::LeftParen,
                Syntax::RightParen,
                Syntax::LeftBrace,
                Syntax::Let_KW,
                Syntax::Ident,
                Syntax::Eq,
                Syntax::Int,
                Syntax::Semi,
                Syntax::RightBrace,
            ]
        );
    }
}
