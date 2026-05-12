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
            $($(pub(crate) const $anchor: Self = Self::$variant;)?)*
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
    ["("] => { super::lexer::Syntax::L_PAREN };
    [")"] => { super::lexer::Syntax::R_PAREN };
    ["{"] => { super::lexer::Syntax::L_BRACE};
    ["}"] => { super::lexer::Syntax::R_BRACE};
    ["["] => { super::lexer::Syntax::L_BRACKET};
    ["]"] => { super::lexer::Syntax::R_BRACKET};
    [" "] => { super::lexer::Syntax::WHITESPACE};
    [?] => { super::lexer::Syntax::MARK};
    [|] => { super::lexer::Syntax::BAR};
    [|=] => { super::lexer::Syntax::BAR_EQ};
    [=] => { super::lexer::Syntax::EQ};
    [==] => { super::lexer::Syntax::EQ2};
    [!=] => { super::lexer::Syntax::NOT_EQ};
    [>] => { super::lexer::Syntax::GT};
    [<] => { super::lexer::Syntax::LT};
    [<=] => { super::lexer::Syntax::LESS_EQ};
    [>=] => { super::lexer::Syntax::GREATER_EQ};
    [,] => { super::lexer::Syntax::COMMA};
    [!] => { super::lexer::Syntax::BANG};
    [;] => { super::lexer::Syntax::SEMI};
    [:] => { super::lexer::Syntax::COLON};
    [.] => { super::lexer::Syntax::DOT};
    [+] => { super::lexer::Syntax::PLUS};
    [+=] => { super::lexer::Syntax::PLUS_EQ};
    [-] => { super::lexer::Syntax::MINUS};
    [-=] => { super::lexer::Syntax::MINUS_EQ};
    [/] => { super::lexer::Syntax::SLASH};
    [/=] => { super::lexer::Syntax::SLASH_EQ};
    ["//"] => { super::lexer::Syntax::SLASH2};
    ["//="] => { super::lexer::Syntax::SLASH2_EQ};
    [*] => { super::lexer::Syntax::STAR};
    [*=] => { super::lexer::Syntax::STAR_EQ};
    [%] => { super::lexer::Syntax::PERCENT};
    [%=] => { super::lexer::Syntax::PERCENT_EQ};
    [->] => { super::lexer::Syntax::ARROW};
    [fn] => { super::lexer::Syntax::FN_KW};
    [mod] => { super::lexer::Syntax::MOD_KW};
    [let] => { super::lexer::Syntax::LET_KW};
    [nil] => { super::lexer::Syntax::NIL_KW};
    [true] => { super::lexer::Syntax::TRUE_KW};
    [false] => { super::lexer::Syntax::FALSE_KW};
    [and] => { super::lexer::Syntax::AND_KW};
    [or] => { super::lexer::Syntax::OR_KW};
    [not] => { super::lexer::Syntax::NOT_KW};
    [return] => { super::lexer::Syntax::RETURN_KW};
    [if] => { super::lexer::Syntax::IF_KW};
    [else] => { super::lexer::Syntax::ELSE_KW};
    [for] => { super::lexer::Syntax::FOR_KW};
    [continue] => { super::lexer::Syntax::CONTINUE_KW};
    [break] => { super::lexer::Syntax::BREAK_KW};
    [while] => { super::lexer::Syntax::WHILE_KW};
    [loop] => { super::lexer::Syntax::LOOP_KW};
    [in] => { super::lexer::Syntax::IN_KW};
    [struct] => { super::lexer::Syntax::STRUCT_KW};
    [enum] => { super::lexer::Syntax::ENUM_KW};
    [impl] => { super::lexer::Syntax::IMPL_KW};
    [match] => { super::lexer::Syntax::MATCH_KW};
    [self] => { super::lexer::Syntax::SELF_KW};
    [Self] => { super::lexer::Syntax::SELF_TYPE_KW};
    [const] => { super::lexer::Syntax::CONST_KW};
    [static] => { super::lexer::Syntax::STATIC_KW};
    [ident] => { super::lexer::Syntax::IDENT }
}

def! {
    #[regex(r"([ \t\n])+")]
    WHITESPACE @WHITESPACE_FIRST,
    #[regex(r"--[^\n\r]*?")]
    COMMENT @WHITESPACE_LAST,

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
    #[token("|=")]
    BAR_EQ = ["|="],
    #[token("=")]
    EQ = ["="],
    #[token("==")]
    EQ2 = ["=="],
    #[token("<")]
    LT,
    #[token(">")]
    GT,
    #[token("<=")]
    LESS_EQ,
    #[token(">=")]
    GREATER_EQ,
    #[token("!=")]
    NOT_EQ = ["!="],
    #[token(",")]
    COMMA = [","],
    #[token("!")]
    BANG = ["!"],
    #[token(".")]
    DOT = ["."],
    #[token("+")]
    PLUS = ["+"],
    #[token("+=")]
    PLUS_EQ = ["+="],
    #[token("-")]
    MINUS = ["-"],
    #[token("-=")]
    MINUS_EQ = ["-="],
    #[token("/")]
    SLASH = ["/="],
    #[token("/=")]
    SLASH_EQ = ["/"],
    #[token("//")]
    SLASH2 = ["//"],
    #[token("//=")]
    SLASH2_EQ = ["//="],
    #[token("*")]
    STAR = ["*"],
    #[token("*=")]
    STAR_EQ = ["*="],
    #[token("%")]
    PERCENT = ["%"],
    #[token("%=")]
    PERCENT_EQ = ["%="],
    #[token(";")]
    SEMI = [";"],
    #[token(":")]
    COLON = [":"],
    #[token("?")]
    MARK = ["?"],
    #[token("->")]
    ARROW = ["->"] @SYMBOL_LAST,

    #[token("let")]
    LET_KW @KEYWORD_FIRST,
    #[token("true")]
    TRUE_KW,
    #[token("false")]
    FALSE_KW,
    #[token("and")]
    AND_KW,
    #[token("or")]
    OR_KW,
    #[token("not")]
    NOT_KW,
    #[token("nil")]
    NIL_KW,
    #[token("return")]
    RETURN_KW,
    #[token("if")]
    IF_KW,
    #[token("else")]
    ELSE_KW,
    #[token("for")]
    FOR_KW,
    #[token("continue")]
    CONTINUE_KW,
    #[token("break")]
    BREAK_KW,
    #[token("while")]
    WHILE_KW,
    #[token("loop")]
    LOOP_KW,
    #[token("in")]
    IN_KW,
    #[token("struct")]
    STRUCT_KW,
    #[token("enum")]
    ENUM_KW,
    #[token("impl")]
    IMPL_KW,
    #[token("match")]
    MATCH_KW,
    #[token("self")]
    SELF_KW,
    #[token("Self")]
    SELF_TYPE_KW,
    #[token("const")]
    CONST_KW,
    #[token("static")]
    STATIC_KW,
    #[token("mod")]
    MOD_KW,
    #[token("fn")]
    FN_KW @KEYWORD_LAST,

    EOF,
    ERROR,

    FILE,

    FN_ITEM,
    MOD_ITEM,

    PATH,
    NAME,
    ARG,
    ARG_LIST,
    PARAM_LIST,
    PARAM,
    RETURN_TYPE,

    LET_STMT,
    EXPR_STMT,

    NILABLE_TYPE,
    LIT_TYPE,
    ANY_TYPE,
    PATH_TYPE,

    NAME_EXPR,
    PATH_EXPR,
    LIT_EXPR,
    PAREN_EXPR,
    ASSIGN_EXPR,
    CALL_EXPR,
    INDEX_EXPR,
    RETURN_EXPR,
    IF_EXPR,
    BLOCK_EXPR,
    BINARY_EXPR,
    UNARY_EXPR,
    TRY_EXPR,
}

impl Syntax {
    pub fn prefix_bp(self) -> Option<u8> {
        Some(match self {
            T![not] => 17,
            T![-] => 18,
            _ => return None,
        })
    }

    pub fn infix_bp(self) -> Option<(u8, u8)> {
        Some(match self {
            T![=] => (1, 2),
            T![or] => (2, 3),
            T![and] => (4, 5),
            T![==] | T![!=] => (6, 7),
            T![<] | T![<=] | T![>] | T![>=] => (8, 9),
            T![+] | T![-] => (10, 11),
            T![*] | T![/] | T![%] => (11, 12),
            _ => return None,
        })
    }

    pub fn postfix_bp(self) -> Option<u8> {
        Some(match self {
            T![?] => 19,
            _ => return None,
        })
    }

    pub fn is_whitespace(self) -> bool {
        (Self::WHITESPACE_FIRST as u16..=Self::WHITESPACE_LAST as u16).contains(&(self as u16))
    }

    pub fn is_keyword(self) -> bool {
        (Self::KEYWORD_FIRST as u16..=Self::KEYWORD_LAST as u16).contains(&(self as u16))
    }

    pub fn is_symbol(self) -> bool {
        (Self::SYMBOL_FIRST as u16..=Self::SYMBOL_LAST as u16).contains(&(self as u16))
    }
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

    #[test]
    fn comment() {
        let lex = Syntax::lexer("--hello there\nlet x = 1")
            .map(|t| t.unwrap())
            .filter(|t| *t != Syntax::WHITESPACE)
            .collect_vec();
        assert_eq!(
            lex.as_slice(),
            &[
                Syntax::COMMENT,
                Syntax::LET_KW,
                Syntax::IDENT,
                Syntax::EQ,
                Syntax::INT
            ]
        );
    }
}
