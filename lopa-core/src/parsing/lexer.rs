use logos::Logos;
use rowan::{TextRange, TextSize};
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
    [~] => { super::lexer::Syntax::TILDE};
    [~=] => { super::lexer::Syntax::TILDE_EQ};
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
    [..] => { super::lexer::Syntax::DOT2};
    [...] => { super::lexer::Syntax::DOT3};
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
    [#] => { super::lexer::Syntax::HASH};
    [@] => { super::lexer::Syntax::AT};
    [%=] => { super::lexer::Syntax::PERCENT_EQ};
    [->] => { super::lexer::Syntax::ARROW};
    [fn] => { super::lexer::Syntax::FN_KW};
    [mod] => { super::lexer::Syntax::MOD_KW};
    [let] => { super::lexer::Syntax::LET_KW};
    [dyn] => { super::lexer::Syntax::DYN_KW};
    [nil] => { super::lexer::Syntax::NIL_KW};
    [root] => { super::lexer::Syntax::ROOT_KW};
    [true] => { super::lexer::Syntax::TRUE_KW};
    [lua] => { super::lexer::Syntax::LUA_KW};
    [use] => { super::lexer::Syntax::USE_KW};
    [false] => { super::lexer::Syntax::FALSE_KW};
    [super] => { super::lexer::Syntax::SUPER_KW};
    [and] => { super::lexer::Syntax::AND_KW};
    [as] => { super::lexer::Syntax::AS_KW};
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
    IDENT = ["identifier"],

    #[regex(r"[\d][\d|_]*\.[\d]+")]
    FLOAT = ["float"],

    #[regex(r"[\d][\d|_]*")]
    INT = ["int"],

    #[regex(r#"""#, lex_string)]
    STRING = ["string"],

    #[regex(r#"'"#, lex_single_string)]
    SINGLE_STRING = ["string"],

    #[regex(r#"\[\["#, lex_bracket_string)]
    BRACKET_STRING = ["string"],

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
    R_BRACE = ["}"],

    #[token("|")]
    BAR = ["|"],

    #[token("|=")]
    BAR_EQ = ["|="],

    #[token("~")]
    TILDE = ["~"],

    #[token("~=")]
    TILDE_EQ = ["~="],

    #[token("=")]
    EQ = ["="],

    #[token("==")]
    EQ2 = ["=="],

    #[token("<")]
    LT = ["<"],

    #[token(">")]
    GT = [">"],

    #[token("<=")]
    LESS_EQ = ["<="],

    #[token(">=")]
    GREATER_EQ = [">="],

    #[token("!=")]
    NOT_EQ = ["!="],

    #[token(",")]
    COMMA = [","],

    #[token("!")]
    BANG = ["!"],

    #[token(".")]
    DOT = ["."],

    #[token("..")]
    DOT2 = [".."],

    #[token("...")]
    DOT3 = ["..."],

    #[token("+")]
    PLUS = ["+"],

    #[token("+=")]
    PLUS_EQ = ["+="],

    #[token("-")]
    MINUS = ["-"],

    #[token("-=")]
    MINUS_EQ = ["-="],

    #[token("/")]
    SLASH = ["/"],

    #[token("/=")]
    SLASH_EQ = ["/="],

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

    #[token("#")]
    HASH = ["#"],

    #[token("@")]
    AT = ["@"],

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
    LET_KW = ["let"]  @KEYWORD_FIRST,

    #[token("dyn")]
    DYN_KW = ["dyn"],

    #[token("true")]
    TRUE_KW = ["true"],

    #[token("false")]
    FALSE_KW = ["false"],

    #[token("super")]
    SUPER_KW = ["super"],

    #[token("and")]
    AND_KW = ["and"],

    #[token("or")]
    OR_KW = ["or"],

    #[token("as")]
    AS_KW = ["as"],

    #[token("not")]
    NOT_KW = ["not"],

    #[token("nil")]
    NIL_KW = ["nil"],

    #[token("return")]
    RETURN_KW = ["return"],

    #[token("if")]
    IF_KW = ["if"],

    #[token("else")]
    ELSE_KW = ["else"],

    #[token("for")]
    FOR_KW = ["for"],

    #[token("continue")]
    CONTINUE_KW = ["continue"],

    #[token("break")]
    BREAK_KW = ["break"],

    #[token("while")]
    WHILE_KW = ["while"],

    #[token("loop")]
    LOOP_KW = ["loop"],

    #[token("in")]
    IN_KW = ["in"],

    #[token("struct")]
    STRUCT_KW = ["struct"],

    #[token("enum")]
    ENUM_KW = ["enum"],

    #[token("impl")]
    IMPL_KW = ["impl"],

    #[token("match")]
    MATCH_KW = ["match"],

    #[token("self")]
    SELF_KW = ["self"],

    #[token("Self")]
    SELF_TYPE_KW = ["Self"],

    #[token("const")]
    CONST_KW = ["const"],

    #[token("static")]
    STATIC_KW = ["static"],

    #[token("mod")]
    MOD_KW = ["mod"],

    #[token("root")]
    ROOT_KW = ["root"],

    #[token("lua")]
    LUA_KW = ["lua"],

    #[token("use")]
    USE_KW = ["use"],

    #[token("fn")]
    FN_KW = ["fn"] @KEYWORD_LAST,

    EOF,
    ERROR,

    MODULE,

    FN_ITEM,
    STRUCT_ITEM,
    IMPL_STRUCT_TYPE,
    IMPL_ITEM,
    MOD_ITEM,
    ENUM_ITEM,
    USE_ITEM,

    PATH,
    NAME,
    ARG,
    PARAM_LIST,
    PARAM,
    FN_TYPE_PARAM_LIST,
    FN_TYPE_PARAM,
    RETURN_TYPE,
    FIELD,
    ENUM_ELEMENT,
    PARENTS_LIST,
    CLOSURE_PARAM_LIST,
    CLOSURE_PARAM,
    RECORD_FIELD,
    USE_TREE_LIST,
    USE_GLOBAL,
    USE_SUPER,
    USE_PATH,
    USE_SELF,
    USE_ROOT,
    USE_NAME,

    LET_STMT,
    EXPR_STMT,

    ITEM_TYPE,
    NILABLE_TYPE,
    PAREN_TYPE,
    LIT_TYPE,
    ANY_TYPE,
    PATH_TYPE,
    FN_TYPE,
    UNIT_TYPE,
    SELF_TYPE,
    DYN_TYPE,

    AS_EXPR,
    SELF_EXPR,
    RECORD_EXPR,
    UNIT_EXPR,
    PATH_EXPR,
    LIT_EXPR,
    PAREN_EXPR,
    ASSIGN_EXPR,
    CALL_EXPR,
    INDEX_EXPR,
    RETURN_EXPR,
    IF_EXPR,
    BLOCK_EXPR,
    LUA_BLOCK_EXPR,
    BINARY_EXPR,
    UNARY_EXPR,
    TRY_EXPR,
    CONTINUE_EXPR,
    BREAK_EXPR,
    CLOSURE_EXPR,
    FIELD_EXPR,
    METHOD_EXPR,

    NAME_PATTERN,

    LUA_BREAK_EXPR,
    LUA_LIT_EXPR,
    LUA_FIELD_INDEX_EXPR,
    LUA_FIELD_CALL_EXPR,
    LUA_INDEX_EXPR,
    LUA_CALL_EXPR,
    LUA_UNARY_EXPR,
    LUA_BINARY_EXPR,
    LUA_STMT_EXPR,
    LUA_MULTI_EXPR,
    LUA_TABLE_EXPR,
    LUA_FIELD_ACCESS_EXPR,
    LUA_FUNCTION_EXPR,

    COMPILER_ATTRIB_LIST,
    COMPILER_ATTRIB,
    COMPILER_ATTRIB_ITEM,

    LUA_ARG_LIST,
    LUA_ARG,
    LUA_PARAM_LIST,
    LUA_PARAM,
    LUA_NAME,

    LUA_FUNCTION_STMT,
    LUA_CONTINUE_STMT,
    LUA_BREAK_STMT,
    LUA_RETURN_STMT,
    LUA_BLOCK_STMT,
    LUA_ASSIGN_STMT,
    LUA_WHILE_STMT,
    LUA_IF_STMT,
    LUA_ELSEIF_STMT,
    LUA_FOR_STMT,
    LUA_LOCAL_STMT,
    LUA_REPEAT_STMT,

}

#[derive(Clone)]
pub struct Lexer<'a> {
    inner: logos::SpannedIter<'a, Syntax>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            inner: Syntax::lexer(input).spanned(),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = LexToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (syntax, span) = self.inner.next()?;
        let range = TextRange::new(TextSize::new(span.start as _), TextSize::new(span.end as _));
        Some(LexToken {
            token: syntax.unwrap_or(Syntax::ERROR),
            text: self.inner.slice(),
            range,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LexToken<'a> {
    pub token: Syntax,
    pub text: &'a str,
    pub range: TextRange,
}

impl Syntax {
    pub fn prefix_bp(self) -> Option<u8> {
        Some(match self {
            T![not] => 15,
            T![-] => 16,
            _ => return None,
        })
    }

    pub fn infix_bp(self) -> Option<(u8, u8)> {
        Some(match self {
            T![=] => (1, 2),
            T![or] => (3, 4),
            T![and] => (5, 6),
            T![==] | T![!=] => (7, 8),
            T![<] | T![<=] | T![>] | T![>=] => (9, 10),
            T![+] | T![-] => (11, 12),
            T![*] | T![/] | T![%] | T!["//"] => (13, 14),
            _ => return None,
        })
    }

    pub fn postfix_bp(self) -> Option<u8> {
        Some(match self {
            T![?] => 17,
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

fn lex_single_string(lex: &mut logos::Lexer<Syntax>) -> bool {
    let rem = lex.remainder();
    let mut len = 0;

    for c in rem.chars() {
        len += c.len_utf8();

        if c == '\'' {
            lex.bump(len);
            return true;
        }
    }
    false
}

fn lex_bracket_string(lex: &mut logos::Lexer<Syntax>) -> bool {
    let rem = lex.remainder();
    let mut len = 0;

    let mut chars = rem.chars().peekable();
    while let Some(c) = chars.next() {
        len += c.len_utf8();

        if c == ']' && chars.peek().cloned() == Some(']') {
            lex.bump(len + 1);
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
