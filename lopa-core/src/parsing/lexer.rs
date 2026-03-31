use logos::Logos;

#[macro_export]
macro_rules! T {
    ['('] => { super::lexer::Syntax::LeftParen };
    [')'] => { super::lexer::Syntax::RightParen };
    ['{'] => { super::lexer::Syntax::LeftBrace };
    ['}'] => { super::lexer::Syntax::RightBrace };
    ['['] => { super::lexer::Syntax::LeftBracket };
    [']'] => { super::lexer::Syntax::RightBracket };
    [' '] => { super::lexer::Syntax::Whitespaces };
    [|] => { super::lexer::Syntax::Bar };
    [=] => { super::lexer::Syntax::Eq };
    [,] => { super::lexer::Syntax::Comma };
    [;] => { super::lexer::Syntax::Semi };
    [:] => { super::lexer::Syntax::Colon };
    [.] => { super::lexer::Syntax::Dot };
    [+] => { super::lexer::Syntax::Plus };
    [-] => { super::lexer::Syntax::Minus };
    [/] => { super::lexer::Syntax::Slash };
    [*] => { super::lexer::Syntax::Star };
    [->] => { super::lexer::Syntax::Arrow };
    [fn] => { super::lexer::Syntax::Fn_KW };
    [let] => { super::lexer::Syntax::Let_KW };
    [ident] => { super::lexer::Syntax::Ident };
    [nil] => { super::lexer::Syntax::Nil_KW };
    [true] => { super::lexer::Syntax::False_KW };
    [false] => { super::lexer::Syntax::True_KW };
    [return] => { super::lexer::Syntax::Return_KW };
    [int] => { super::lexer::Syntax::Int };
    [float] => { super::lexer::Syntax::Float };
    [eof] => { super::lexer::Syntax::EndOfFile };
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
    #[token("nil")]
    Nil_KW,
    #[token("return")]
    Return_KW,
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
