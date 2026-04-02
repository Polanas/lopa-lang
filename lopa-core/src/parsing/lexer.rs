use logos::Logos;
use std::fmt;

macro_rules! def {
    (
      $(
        $(#[$meta:meta])*
        $variant: ident $(= [$($tt:tt)*])? $(@ $anchor:ident)?
      ),* $(,)?
    ) => {
        #[allow(non_camel_case_types)]
        #[derive(Logos, Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
        #[repr(u16)]
        pub enum Syntax {
            $(
                $(#[$meta])*
                $variant,
            )*
        }

        impl Syntax {
            $($(const $anchor: Self = Self::$variant;)?)*
        }

        impl fmt::Display for Syntax {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(Self::$variant => f.write_str(to_str!($variant, $($($tt)*)?)),)*
                }
            }
        }
    };
}

macro_rules! to_str {
    // IDENT
    ($variant:tt, ) => {
        stringify!($variant)
    };
    // Special case.
    ($variant:tt, '"') => {
        r#"""#
    };
    // This breaks `literal` fragment.
    ($variant:tt, -) => {
        r#"-"#
    };
    // '['
    ($variant:tt, $s:literal) => {
        $s
    };
    // &&
    ($variant:tt, $($tt:tt)+) => {
        stringify!($($tt)+)
    };
}

#[macro_export]
macro_rules! T {
    ['('] => { super::lexer::Syntax::L_PAREN };
    [')'] => { super::lexer::Syntax::R_PAREN };
    ['{'] => { super::lexer::Syntax::L_BRACE};
    ['}'] => { super::lexer::Syntax::R_BRACE};
    ['['] => { super::lexer::Syntax::L_BRACKET};
    [']'] => { super::lexer::Syntax::R_BRACKET};
    [' '] => { super::lexer::Syntax::WHITESPACE};
    [|] => { super::lexer::Syntax::BAR};
    [=] => { super::lexer::Syntax::EQ};
    [,] => { super::lexer::Syntax::COMMA};
    [;] => { super::lexer::Syntax::SEMI};
    [:] => { super::lexer::Syntax::COLON};
    [.] => { super::lexer::Syntax::DOT};
    [+] => { super::lexer::Syntax::PLUS};
    [-] => { super::lexer::Syntax::MINUS};
    [/] => { super::lexer::Syntax::SLASH};
    [*] => { super::lexer::Syntax::STAR};
    [->] => { super::lexer::Syntax::ARROW};
    [fn] => { super::lexer::Syntax::FN_KW};
    [let] => { super::lexer::Syntax::LET_KW};
    [ident] => { super::lexer::Syntax::IDENT};
    [nil] => { super::lexer::Syntax::NIL_KW};
    [true] => { super::lexer::Syntax::TRUE_KW};
    [false] => { super::lexer::Syntax::FALSE_KW};
    [return] => { super::lexer::Syntax::RETURN_KW};
    [int] => { super::lexer::Syntax::INT};
    [float] => { super::lexer::Syntax::FLOAT};
    [eof] => { super::lexer::Syntax::EOF };
}

def! {
    #[regex(r"([ \t\n])+")]
    WHITESPACE @WHITESPACE_FIRST,

    #[regex(r"[_]?[A-Za-z_][0-9A-Za-z_]*")]
    IDENT,
    #[regex(r"[\d][\d|_]*\.[\d]+")]
    FLOAT,
    #[regex(r"[\d][\d|_]*")]
    INT,
    #[regex(r#"""#, lex_string)]
    STRING,

    #[token("(")]
    L_PAREN = ["("] @SYMBOL_FIRST,
    #[token(")")]
    R_PAREN = [")"],
    #[token("[")]
    L_BRACKET = ["["],
    #[token("]")]
    R_BRACKET = ["]"],
    #[token("{")]
    L_BRACE = ["{"],
    #[token("}")]
    R_BRACE = ["{"],
    #[token("|")]
    BAR = ["|"],
    #[token("=")]
    EQ = ["="],
    #[token(",")]
    COMMA = [","],
    #[token(".")]
    DOT = ["."],
    #[token("+")]
    PLUS = ["+"],
    #[token("-")]
    MINUS = ["-"],
    #[token("/")]
    SLASH = ["/"],
    #[token("*")]
    STAR = ["*"],
    #[token(";")]
    SEMI = [";"],
    #[token(":")]
    COLON = [":"],
    #[token("->")]
    ARROW = ["->"] @SYMBOL_LAST,

    #[token("let")]
    LET_KW @KEYWORD_FIRST,
    #[token("true")]
    TRUE_KW,
    #[token("false")]
    FALSE_KW,
    #[token("nil")]
    NIL_KW,
    #[token("return")]
    RETURN_KW,
    #[token("fn")]
    FN_KW @KEYWORD_LAST,

    EOF,
    ERROR,

    FILE,

    FN_ITEM,

    ARG,
    ARG_LIST,
    PARAM_LIST,
    PARAM,
    RETURN_TYPE,

    LET_STMT,
    EXPR_STMT,

    TYPE_EXPR,
    LIT_EXPR,
    PAREN_EXPR,
    ASSIGN_EXPR,
    CALL_EXPR,
    INDEX_EXPR,
    RETURN_EXPR,
    BLOCK_EXPR,
    BINARY_EXPR,
}

fn lex_string(lex: &mut logos::Lexer<Syntax>) -> bool {
    let rem = lex.remainder();
    let mut len = 0;

    for c in rem.chars() {
        len += c.len_utf8();

        if c == '"' {
            lex.bump(len);
            return true;
        }
    }
    false
}

// #[allow(non_camel_case_types)]
// #[derive(Logos, Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
// #[repr(u16)]
// pub enum Syntax {
//     #[token("(")]
//     LeftParen,
//     #[token(")")]
//     RightParen,
//     #[token("[")]
//     LeftBracket,
//     #[token("]")]
//     RightBracket,
//     #[token("{")]
//     LeftBrace,
//     #[token("}")]
//     RightBrace,
//     #[token("|")]
//     Bar,
//     #[token("=")]
//     Eq,
//     #[token(",")]
//     Comma,
//     #[token(".")]
//     Dot,
//     #[token("+")]
//     Plus,
//     #[token("-")]
//     Minus,
//     #[token("/")]
//     Slash,
//     #[token("*")]
//     Star,
//     #[token(";")]
//     Semi,
//     #[token(":")]
//     Colon,
//     #[token("let")]
//     Let_KW,
//     #[token("true")]
//     True_KW,
//     #[token("false")]
//     False_KW,
//     #[token("nil")]
//     Nil_KW,
//     #[token("return")]
//     Return_KW,
//     #[token("fn")]
//     Fn_KW,
//     #[token("->")]
//     Arrow,
//     #[regex(r"[_]?[A-Za-z_][0-9A-Za-z_]*")]
//     Ident,
//     #[regex(r"[\d][\d|_]*")]
//     Int,
//     #[regex(r"[\d][\d|_]*\.[\d]+")]
//     Flo
//     #[regex(r"\s+")]
//     Whitespaces,
//     Error,
//     #[end]
//     EndOfFile,
// }

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
            .filter(|t| *t != Syntax::WHITESPACE)
            .collect_vec();

        assert_eq!(
            lex.as_slice(),
            &[
                Syntax::FN_KW,
                Syntax::IDENT,
                Syntax::L_PAREN,
                Syntax::R_PAREN,
                Syntax::L_BRACE,
                Syntax::LET_KW,
                Syntax::IDENT,
                Syntax::EQ,
                Syntax::INT,
                Syntax::SEMI,
                Syntax::R_BRACE,
            ]
        );
    }
}
