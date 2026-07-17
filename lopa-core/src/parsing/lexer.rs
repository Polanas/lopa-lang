use logos::Logos;
use std::{fmt, ops::Range};

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
    ["("] => { $crate::parsing::lexer::Syntax::L_PAREN };
    [")"] => { $crate::parsing::lexer::Syntax::R_PAREN };
    ["{"] => { $crate::parsing::lexer::Syntax::L_BRACE};
    ["}"] => { $crate::parsing::lexer::Syntax::R_BRACE};
    ["["] => { $crate::parsing::lexer::Syntax::L_BRACKET};
    ["]"] => { $crate::parsing::lexer::Syntax::R_BRACKET};
    [" "] => { $crate::parsing::lexer::Syntax::WHITESPACE};
    [_] => { $crate::parsing::lexer::Syntax::WILDCARD};
    [?] => { $crate::parsing::lexer::Syntax::MARK};
    [|] => { $crate::parsing::lexer::Syntax::BAR};
    [|=] => { $crate::parsing::lexer::Syntax::BAR_EQ};
    [~] => { $crate::parsing::lexer::Syntax::TILDE};
    [~=] => { $crate::parsing::lexer::Syntax::TILDE_EQ};
    [=] => { $crate::parsing::lexer::Syntax::EQ};
    [==] => { $crate::parsing::lexer::Syntax::EQ2};
    [!=] => { $crate::parsing::lexer::Syntax::NOT_EQ};
    [>] => { $crate::parsing::lexer::Syntax::GT};
    [<] => { $crate::parsing::lexer::Syntax::LT};
    [<=] => { $crate::parsing::lexer::Syntax::LESS_EQ};
    [>=] => { $crate::parsing::lexer::Syntax::GREATER_EQ};
    [,] => { $crate::parsing::lexer::Syntax::COMMA};
    [!] => { $crate::parsing::lexer::Syntax::BANG};
    [;] => { $crate::parsing::lexer::Syntax::SEMI};
    [:] => { $crate::parsing::lexer::Syntax::COLON};
    [.] => { $crate::parsing::lexer::Syntax::DOT};
    [..] => { $crate::parsing::lexer::Syntax::DOT2};
    [...] => { $crate::parsing::lexer::Syntax::DOT3};
    [+] => { $crate::parsing::lexer::Syntax::PLUS};
    [+=] => { $crate::parsing::lexer::Syntax::PLUS_EQ};
    [-] => { $crate::parsing::lexer::Syntax::MINUS};
    [-=] => { $crate::parsing::lexer::Syntax::MINUS_EQ};
    [/] => { $crate::parsing::lexer::Syntax::SLASH};
    [/=] => { $crate::parsing::lexer::Syntax::SLASH_EQ};
    ["//"] => { $crate::parsing::lexer::Syntax::SLASH2};
    ["//="] => { $crate::parsing::lexer::Syntax::SLASH2_EQ};
    [*] => { $crate::parsing::lexer::Syntax::STAR};
    [*=] => { $crate::parsing::lexer::Syntax::STAR_EQ};
    [%] => { $crate::parsing::lexer::Syntax::PERCENT};
    [#] => { $crate::parsing::lexer::Syntax::HASH};
    [^] => { $crate::parsing::lexer::Syntax::CARET};
    [@] => { $crate::parsing::lexer::Syntax::AT};
    [%=] => { $crate::parsing::lexer::Syntax::PERCENT_EQ};
    [->] => { $crate::parsing::lexer::Syntax::ARROW};
    [=>] => { $crate::parsing::lexer::Syntax::FAT_ARROW};
    [fn] => { $crate::parsing::lexer::Syntax::FN_KW};
    [mod] => { $crate::parsing::lexer::Syntax::MOD_KW};
    [let] => { $crate::parsing::lexer::Syntax::LET_KW};
    [dyn] => { $crate::parsing::lexer::Syntax::DYN_KW};
    [nil] => { $crate::parsing::lexer::Syntax::NIL_KW};
    [root] => { $crate::parsing::lexer::Syntax::ROOT_KW};
    [true] => { $crate::parsing::lexer::Syntax::TRUE_KW};
    [lua] => { $crate::parsing::lexer::Syntax::LUA_KW};
    [use] => { $crate::parsing::lexer::Syntax::USE_KW};
    [false] => { $crate::parsing::lexer::Syntax::FALSE_KW};
    [super] => { super::lexer::Syntax::SUPER_KW};
    [and] => { $crate::parsing::lexer::Syntax::AND_KW};
    [as] => { $crate::parsing::lexer::Syntax::AS_KW};
    [is] => { $crate::parsing::lexer::Syntax::IS_KW};
    [!is] => { $crate::parsing::lexer::Syntax::IS_NOT_KW};
    [is_not] => { $crate::parsing::lexer::Syntax::IS_NOT_KW};
    [not] => { $crate::parsing::lexer::Syntax::NOT_KW};
    [or] => { $crate::parsing::lexer::Syntax::OR_KW};
    [return] => { $crate::parsing::lexer::Syntax::RETURN_KW};
    [if] => { $crate::parsing::lexer::Syntax::IF_KW};
    [else] => { $crate::parsing::lexer::Syntax::ELSE_KW};
    [for] => { $crate::parsing::lexer::Syntax::FOR_KW};
    [continue] => { $crate::parsing::lexer::Syntax::CONTINUE_KW};
    [break] => { $crate::parsing::lexer::Syntax::BREAK_KW};
    [while] => { $crate::parsing::lexer::Syntax::WHILE_KW};
    [loop] => { $crate::parsing::lexer::Syntax::LOOP_KW};
    [in] => { $crate::parsing::lexer::Syntax::IN_KW};
    [struct] => { $crate::parsing::lexer::Syntax::STRUCT_KW};
    [enum] => { $crate::parsing::lexer::Syntax::ENUM_KW};
    [impl] => { $crate::parsing::lexer::Syntax::IMPL_KW};
    [match] => { $crate::parsing::lexer::Syntax::MATCH_KW};
    [self] => { $crate::parsing::lexer::Syntax::SELF_KW};
    [Self] => { $crate::parsing::lexer::Syntax::SELF_TYPE_KW};
    [const] => { $crate::parsing::lexer::Syntax::CONST_KW};
    [static] => { $crate::parsing::lexer::Syntax::STATIC_KW};
    [ident] => { $crate::parsing::lexer::Syntax::IDENT }
}

impl From<Syntax> for u16 {
    fn from(value: Syntax) -> Self {
        value as u16
    }
}

def! {
    #[regex(r"([ \t\n])+")]
    WHITESPACE @WHITESPACE_FIRST,

    #[regex(r"--[^\n\r]*?")]
    COMMENT @WHITESPACE_LAST,

    #[regex(r"[0-9A-Za-z_][0-9A-Za-z_]*", priority=1)]
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

    #[token("_")]
    WILDCARD = ["_"],

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

    #[token("^")]
    CARET = ["^"],

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

    #[token("=>")]
    FAT_ARROW = ["=>"],

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

    #[token("is")]
    IS_KW = ["is"],

    #[token("not")]
    NOT_KW = ["not"],

    #[token("!is")]
    IS_NOT_KW = ["!is"],

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

    FN_ITEM @ITEM_FIRST,
    STRUCT_ITEM,
    IMPL_ITEM,
    MOD_ITEM,
    ENUM_ITEM,
    USE_ITEM @ITEM_LAST,

    PATH,
    NAME,
    ARG,
    PARAM_LIST,
    PARAM,
    FN_TYPE_PARAM_LIST,
    FN_TYPE_PARAM,
    RETURN_TYPE,
    FIELD,
    PARENT,
    CLOSURE_PARAM_LIST,
    CLOSURE_PARAM,
    RECORD_FIELD,
    GENERICS,
    GENERIC_ARGUMENTS,
    TYPE_PARAM,
    TYPE_PARAM_BOUND,
    PATH_SEGMENT,

    USE_ROOT_PATH @USE_FIRST,
    USE_SUPER_PATH,
    USE_SELF_NAME,
    USE_PATH,
    USE_NAME,
    USE_GLOBAL,
    USE_TREE_LIST @USE_LAST,

    LET_STMT @STMT_FIRST,
    EXPR_STMT @STMT_LAST,

    NILABLE_TYPE @TYPE_EXPR_FIRST,
    PAREN_TYPE,
    LIT_TYPE,
    LIT_TYPE_STRING,
    LIT_TYPE_INT,
    LIT_TYPE_FLOAT,
    LIT_TYPE_BOOL,
    ANY_TYPE,
    PATH_TYPE,
    FN_TYPE,
    UNIT_TYPE,
    TUPLE_TYPE,
    SELF_TYPE,
    DYN_TYPE @TYPE_EXPR_LAST,

    SELF_EXPR @EXPR_FIRST,
    RECORD_EXPR,
    UNIT_EXPR,
    TUPLE_EXPR,
    PATH_EXPR,
    LIT_EXPR,
    PAREN_EXPR,
    ASSIGN_EXPR,
    CALL_EXPR,
    INDEX_EXPR,
    RETURN_EXPR,
    IF_EXPR,
    FOR_EXPR,
    WHILE_EXPR,
    LOOP_EXPR,
    BLOCK_EXPR,
    LUA_CHUNK_EXPR,
    BINARY_EXPR,
    UNARY_EXPR,
    CONTINUE_EXPR,
    BREAK_EXPR,
    CLOSURE_EXPR,
    METHOD_EXPR,
    SAFE_METHOD_EXPR,
    FIELD_EXPR,
    SAFE_FIELD_EXPR,
    AS_EXPR,
    IS_EXPR,
    IS_NOT_EXPR @EXPR_LAST,

    NAME_PAT @PAT_FIRST,
    PATH_PAT,
    LIT_PAT,
    WILDCARD_PAT @PAT_LAST,

    COMPILER_ATTRIB_LIST,
    COMPILER_ATTRIB,
    COMPILER_ATTRIB_ITEM,

    LUA_LIT_EXPR @LUA_EXPR_FIRST,
    LUA_INDEX_EXPR,
    LUA_CALL_EXPR,
    LUA_UNARY_EXPR,
    LUA_BINARY_EXPR,
    LUA_MULTI_EXPR,
    LUA_TABLE_EXPR,
    LUA_FIELD_ACCESS_EXPR,
    LUA_FUNCTION_EXPR @LUA_EXPR_LAST,

    LUA_ELEM_ASSIGN @LUA_ELEM_FIRST,
    LUA_ELEM_INDEX_ASSIGN,
    LUA_ELEM_EXPR @LUA_ELEM_LAST,

    LUA_ARG_LIST,
    LUA_ARG,
    LUA_PARAM_LIST,
    LUA_PARAM,
    LUA_NAME,
    LUA_HASH_NAME,
    LUA_NUMERIC_FOR,
    LUA_GENERIC_FOR,

    LUA_FUNCTION_STMT @LUA_STMT_FIRST,
    LUA_CONTINUE_STMT,
    LUA_BREAK_STMT,
    LUA_RETURN_STMT,
    LUA_HASH_RETURN_STMT,
    LUA_BLOCK_STMT,
    LUA_STMT_EXPR,
    LUA_WHILE_STMT,
    LUA_IF_STMT,
    LUA_FOR_STMT,
    LUA_LOCAL_STMT,
    LUA_REPEAT_STMT @LUA_STMT_LAST,

    LUA_ELSE,
    LUA_ELSEIF,
}

#[derive(Clone)]
pub(super) struct Lexer<'a> {
    inner: logos::SpannedIter<'a, Syntax>,
}

impl<'a> Lexer<'a> {
    pub(super) fn new(input: &'a str) -> Self {
        Self {
            inner: Syntax::lexer(input).spanned(),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = LexToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (syntax, span) = self.inner.next()?;
        Some(LexToken {
            token: syntax.unwrap_or(Syntax::ERROR),
            text: self.inner.slice(),
            range: span,
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct LexToken<'a> {
    pub(super) token: Syntax,
    pub(super) text: &'a str,
    pub(super) range: Range<usize>,
}

impl Syntax {
    pub fn prefix_bp(self) -> Option<u8> {
        Some(match self {
            T![!] => 15,
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

    pub fn is_whitespace(self) -> bool {
        (Self::WHITESPACE_FIRST as u16..=Self::WHITESPACE_LAST as u16).contains(&(self as u16))
    }

    pub fn is_keyword(self) -> bool {
        (Self::KEYWORD_FIRST as u16..=Self::KEYWORD_LAST as u16).contains(&(self as u16))
    }

    pub fn is_symbol(self) -> bool {
        (Self::SYMBOL_FIRST as u16..=Self::SYMBOL_LAST as u16).contains(&(self as u16))
    }

    pub fn is_item(self) -> bool {
        (Self::ITEM_FIRST as u16..=Self::ITEM_LAST as u16).contains(&(self as u16))
    }

    pub fn is_elem(self) -> bool {
        matches!(self, Self::FIELD | Self::FN_ITEM)
    }

    pub fn is_stmt(self) -> bool {
        (Self::STMT_FIRST as u16..=Self::STMT_LAST as u16).contains(&(self as u16))
    }

    pub fn is_expr(self) -> bool {
        (Self::EXPR_FIRST as u16..=Self::EXPR_LAST as u16).contains(&(self as u16))
    }
    pub fn is_use(self) -> bool {
        (Self::USE_FIRST as u16..=Self::USE_LAST as u16).contains(&(self as u16))
    }

    pub fn is_pattern(self) -> bool {
        (Self::PAT_FIRST as u16..=Self::PAT_LAST as u16).contains(&(self as u16))
    }

    pub fn is_type_expr(self) -> bool {
        (Self::TYPE_EXPR_FIRST as u16..=Self::TYPE_EXPR_LAST as u16).contains(&(self as u16))
    }

    pub fn is_lua_stmt(self) -> bool {
        (Self::LUA_STMT_FIRST as u16..=Self::LUA_STMT_LAST as u16).contains(&(self as u16))
    }

    pub fn is_lua_expr(self) -> bool {
        (Self::LUA_EXPR_FIRST as u16..=Self::LUA_EXPR_LAST as u16).contains(&(self as u16))
    }

    pub fn is_lua_elem(self) -> bool {
        (Self::LUA_ELEM_FIRST as u16..=Self::LUA_ELEM_LAST as u16).contains(&(self as u16))
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
