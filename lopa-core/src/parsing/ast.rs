use super::{lexer::Syntax, parser};
use crate::T;
use crate::common::LitKind;
use rowan::NodeOrToken;
use rowan::ast::support::{child, children, token};
use rowan::ast::{AstChildren, AstNode};
use ustr::Ustr;

pub type SyntaxNode = rowan::SyntaxNode<parser::Lang>;
pub type SyntaxToken = rowan::SyntaxToken<parser::Lang>;
pub type SyntaxNodePtr = rowan::ast::SyntaxNodePtr<parser::Lang>;
pub type AstPtr<N> = rowan::ast::AstPtr<N>;

//used for the matches! check in can_cast for enums
trait NodeWrapper {
    const KIND: Syntax;
}

pub trait HasCompilerAttribs: AstNode<Language = parser::Lang> {
    fn attribs(&self) -> Option<CompilerAttribList> {
        self.syntax()
            .prev_sibling()
            .and_then(CompilerAttribList::cast)
    }
}
impl<T: AstNode<Language = parser::Lang>> HasCompilerAttribs for T {}

macro_rules! structs {
    (
      $(
          $kind:ident = $name:ident $([$trait:tt])?
          { $($impl:tt)* } $(,)?
      ),*
    ) => {
        $(
            #[derive(Clone, PartialEq, Eq, Hash, Debug, salsa::Update)]
            pub struct $name(pub SyntaxNode);

            impl $name {
                struct_impl!($($impl)*);

                pub fn node_ptr(&self) -> SyntaxNodePtr {
                    SyntaxNodePtr::new(&self.0)
                }
            }

            impl NodeWrapper for $name {
                const KIND: Syntax = Syntax::$kind;
            }

            $(impl $trait for $name{})*

            impl AstNode for $name {
                type Language = parser::Lang;

                fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
                where
                    Self: Sized,
                {
                    kind == Syntax::$kind
                }

                fn cast(syntax: SyntaxNode) -> Option<Self>
                where
                    Self: Sized,
                {
                    Self::can_cast(syntax.kind()).then(|| Self(syntax))
                }

                fn syntax(&self) -> &SyntaxNode {
                    &self.0
                }
            }
        )*
    };
}

macro_rules! struct_impl {
    () => {};
    //regular child, any
    ($field:ident: $ast:ident, $($tt:tt)*) => {
        pub fn $field(&self) -> Option<$ast> {
            child(&self.0)
        }
        struct_impl!($($tt)*);
    };
    //child with an offset
    ($field:ident[$k:tt]: $ast:ident, $($tt:tt)*) => {
        pub fn $field(&self) -> Option<$ast> { children(&self.0).nth($k) }
        struct_impl!($($tt)*);
    };
    //list of children
    ($field:ident: [$ast:ident], $($tt:tt)*) => {
        pub fn $field(&self) -> AstChildren<$ast> {
            children(&self.0)
        }
        struct_impl!($($tt)*);
    };
    //token
    ($field:ident: T![$tok:tt], $($tt:tt)*) => {

        pub fn $field(&self) -> Option<SyntaxToken> {
            token(&self.0, T![$tok])
        }
        struct_impl!($($tt)*);
    };
    //token with an offset
    ($field:ident[$k:tt]: T![$tok:tt], $($tt:tt)*) => {

        pub fn $field(&self) -> Option<SyntaxToken> {
            self.syntax().children_with_tokens().filter_map(|i| i.into_token()).nth($k)
        }
        struct_impl!($($tt)*);
    };
    ($($item:item)*) => {
        $($item)*
    }
}

macro_rules! enums {
    (
        $(
            $name:ident {
                $(
                    $variant:ident$(<$generic:ident>)?
                ),* $(,)?
            }
        ),* $(,)?
    ) => {
        $(
            #[derive(Clone, Debug, PartialEq, Eq, Hash, salsa::Update)]
            pub enum $name {
                $($variant($variant$(<$generic>)*),)*
            }

            impl $name {
                pub fn node_ptr(&self) -> SyntaxNodePtr {
                    match self {
                        $(Self::$variant(e) => e.node_ptr()),*
                    }
                }
            }

            impl AstNode for $name {
                type Language = parser::Lang;

                fn can_cast(kind: <Self::Language as rowan::Language>::Kind) -> bool
                where
                    Self: Sized,
                {
                    matches!(kind, $(<$variant as NodeWrapper>::KIND)|*)
                }

                fn cast(syntax: SyntaxNode) -> Option<Self>
                where
                    Self: Sized,
                {
                    match syntax.kind() {
                        $(<$variant as NodeWrapper>::KIND => Some(Self::$variant$(::<$generic>)*($variant(syntax))),)*
                        _ => None,
                    }
                }

                fn syntax(&self) -> &SyntaxNode {
                    match self {
                        $(Self::$variant(e) => &e.0,)*
                    }
                }
            }
        )*
    };
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum BinaryOpKind {
    Add,
    Mul,
    Div,
    DivInt,
    Rem,
    Or,
    Shl,
    Shr,
    BitXor,
    BitAnd,
    Sub,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
    Equal,
    And,
    BitOr,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    DivIntAssign,
    RemAssign,
    BitXorAssign,
    BitAndAssign,
    BitOrAssign,
    ShlAssign,
    ShrAssign,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum UnaryOpKind {
    Not,
    Neg,
}

structs! {
    FILE = File {
        items: [Item],
    },
    MOD_ITEM = ModItem {
        mod_token: T![mod],
        semi: T![;],
        items: [FnItem],
    },
    STRUCT_ITEM = StructItem {
        struct_token: T![struct],
        name: Name,
        fields: FieldList,
    },
    FIELD_LIST = FieldList {
        left_brace_token: T!["{"],
        fields: [Field],
        right_brace_token: T!["}"],
    },
    FIELD = Field {
        name: Name,
        colon_token: T![:],
        ty: TypeExpr,
        eq_token: T![=],
        default_value: Expr,
    },
    FN_ITEM = FnItem {
        fn_token: T![fn],
        name: Name,
        params: ParamList,
        output: ReturnType,
        body: BlockExpr,
    },
    RETURN_TYPE = ReturnType {
        arrow_token: T![->],
        ty: TypeExpr,
    },
    PARAM_LIST = ParamList {
        left_paren_token: T!["("],
        params: [FnParam],
        right_paren_token: T![")"],
    },
    PARAM = FnParam {
        self_token: T![self],
        pattern: Pattern,
        colon_token: T![:],
        ty: TypeExpr,
        eq_token: T![=],
        default_value: Expr,
    },
    EXPR_STMT = ExprStmt {
        expr: Expr,
        semi_token: T![;],
        right_paren_token: T![")"],
    },
    LET_STMT = LetStmt {
        let_token: T![let],
        pattern: Pattern,
        colon_token: T![:],
        ty: TypeExpr,
        eq_token: T![=],
        expr: Expr,
        semi: T![;],
    },
    NAME_PATTERN = NamePattern {
        name: Name,
    },

    NILABLE_TYPE = NilableType {
        ty: TypeExpr,
        mark_token: T![?],
    },
    ANY_TYPE = AnyType { },
    UNIT_TYPE = UnitType {},
    LIT_TYPE = LitType {
        pub fn kind(&self) -> Option<LitKind> {
            let token = self.syntax().first_token()?;
            let Syntax::IDENT = token.kind() else {
                return None;
            };
            Some(match token.text() {
                "string" => LitKind::String,
                "int" => LitKind::Int,
                "float" => LitKind::Float,
                "bool" => LitKind::Bool,
                _ => return None,
            })
        }
    },
    FN_TYPE = FnType {
        fn_keyword: T![fn],
        param_list: FnTypeParamList,
        output: ReturnType,
    },
    FN_TYPE_PARAM_LIST = FnTypeParamList {
        left_paren_token: T!["("],
        params: [FnTypeParam],
        right_paren_token: T![")"],
    },
    FN_TYPE_PARAM = FnTypeParam {
        name: Name,
        colon_token: T![:],
        ty: TypeExpr,
    },
    PATH_TYPE = PathType {
        value: Path,
    },
    UNARY_EXPR = UnaryExpr {
        expr: Expr,

        pub fn op_token(&self) -> Option<SyntaxToken> {
            self.op_details().map(|t| t.0)
        }

        pub fn op_kind(&self) -> Option<UnaryOpKind> {
            self.op_details().map(|t| t.1)
        }

        pub fn op_details(&self) -> Option<(SyntaxToken, UnaryOpKind)> {
            self.syntax().children_with_tokens().find_map(|c| {
                let token = c.into_token()?;
                let op = match token.kind() {
                    T![-] => UnaryOpKind::Neg,
                    T![!] => UnaryOpKind::Not,
                    _ => return None,
                };
                Some((token, op))
            })
        }
    },
    UNIT_EXPR = UnitExpr {
    },
    PATH_EXPR = PathExpr {
        path: Path,
    },
    BINARY_EXPR = BinaryExpr {
        lhs: Expr,
        rhs[1]: Expr,

        pub fn op_token(&self) -> Option<SyntaxToken> {
            self.op_details().map(|t| t.0)
        }

        pub fn op_kind(&self) -> Option<BinaryOpKind> {
            self.op_details().map(|t| t.1)
        }

        pub fn op_details(&self) -> Option<(SyntaxToken, BinaryOpKind)> {
            self.syntax().children_with_tokens().find_map(|c| {
                let token = c.into_token()?;
                let op = match token.kind() {
                    T![+] => BinaryOpKind::Add,
                    T![*] => BinaryOpKind::Mul,
                    T![/] => BinaryOpKind::Div,
                    T!["//"] => BinaryOpKind::DivInt,
                    T![%] => BinaryOpKind::Rem,
                    T![or] => BinaryOpKind::Or,
                    // T![] => BinaryOpKind::Shl,
                    // T![] => BinaryOpKind::Shr,
                    // T![] => BinaryOpKind::BitXor,
                    // T![] => BinaryOpKind::BitAnd,
                    T![-] => BinaryOpKind::Sub,
                    T![>] => BinaryOpKind::Greater,
                    T![>=] => BinaryOpKind::GreaterEqual,
                    T![<] => BinaryOpKind::Less,
                    T![<=] => BinaryOpKind::LessEqual,
                    T![!=] => BinaryOpKind::NotEqual,
                    T![==] => BinaryOpKind::Equal,
                    T![and] => BinaryOpKind::And,
                    T![|] => BinaryOpKind::BitOr,
                    T![+=] => BinaryOpKind::AddAssign,
                    T![-=] => BinaryOpKind::SubAssign,
                    T![*=] => BinaryOpKind::MulAssign,
                    T![/=] => BinaryOpKind::DivAssign,
                    T!["//="] => BinaryOpKind::DivIntAssign,
                    T![%=] => BinaryOpKind::RemAssign,
                    // T![] => BinaryOpKind::BitXorAssign,
                    // T![] => BinaryOpKind::BitAndAssign,
                    // T![] => BinaryOpKind::BitOrAssign,
                    // T![] => BinaryOpKind::ShlAssign,
                    // T![] => BinaryOpKind::ShrAssign,
                    _ => return None,
                };
                Some((token, op))
            })
        }
    },
    CLOSURE_EXPR = ClosureExpr {
        bar_left_token: T![|],
        params: ClosureParamList,
        bar_right_token[1]: T![|],
    },
    CLOSURE_PARAM_LIST = ClosureParamList {
        params: [ClosureParam],
    },
    CLOSURE_PARAM = ClosureParam {
        pattern: Pattern,
        colon_token: T![:],
        ty: TypeExpr,
    },
    RETURN_EXPR = ReturnExpr {
        return_token: T![return],
        expr: Expr,
    },
    INDEX_EXPR = IndexExpr {
        base: Expr,
        left_bracket_token: T!["["],
        index[1]: Expr,
        right_bracket_token: T!["]"],
    },
    ARG_LIST = ArgList {
        left_paren_token: T!["("],
        args: [Arg],
        right_paren_token: T![")"],
    },
    ARG = Arg {
        label: Name,
        colon_token: T![:],
        value: Expr,
    },
    CALL_EXPR = CallExpr {
        func: Expr,
        args: ArgList,
    },
    PAREN_EXPR = ParenExpr {
        left_paren_token: T!["("],
        expr: Expr,
        right_paren_token: T![")"],
    },
    BLOCK_EXPR = BlockExpr {
        left_curly: T!["{"],
        stmts: [Stmt],
        right_curly: T!["}"],
    },
    IF_EXPR = IfExpr {
        if_token: T![if],
        if_condition: Expr,
        if_branch: BlockExpr,
        else_token: T![else],
        else_branch[1]: BlockExpr,
        else_if_expr: IfExpr,
    },
    FIELD_EXPR = FieldExpr {
        expr: Expr,
        dot_token: T![.],
        name: Name,
    },
    METHOD_EXPR = MethodExpr {
        expr: Expr,
        dot_token: T![.],
        name: Name,
        args: ArgList,

    },
    RECORD_EXPR = RecordExpr {
        path: Path,
        list: RecordFieldList,
    },
    RECORD_FIELD_LIST = RecordFieldList {
        left_brace_token: T!["{"],
        fields: [RecordField],
        right_brace_token: T!["}"],
    },
    RECORD_FIELD = RecordField {
        name: Name,
        colon_token: T![:],
        exrp: Expr,
    },
    TRY_EXPR = TryExpr {
        expr: Expr,
        mark_token: T![?],
    },
    LIT_EXPR = LitExpr {
        pub fn token(&self) -> Option<SyntaxToken> {
            self.0.children_with_tokens().find_map(NodeOrToken::into_token)
        }

        pub fn text(&self) -> Option<Ustr> {
            self.token().map(|t| t.text().into())
        }

        pub fn kind(&self) -> Option<LitKind> {
            Some(match self.token()?.kind() {
                Syntax::INT => LitKind::Int,
                Syntax::FLOAT => LitKind::Float,
                Syntax::STRING => LitKind::String,
                Syntax::TRUE_KW | Syntax::FALSE_KW => LitKind::Bool,
                _ => return None,
            })
        }
    },
    PATH = Path {
        pub fn segments(&self) -> impl Iterator<Item = Ustr> {
            self.0.children_with_tokens().filter_map(|t| t.into_token()).filter(|t| t.kind() == Syntax::IDENT)
                .map(|t| Ustr::from(t.text()))
        }
    },
    COMPILER_ATTRIB_LIST = CompilerAttribList {
        attribs: [CompilerAttrib],
    },
    COMPILER_ATTRIB = CompilerAttrib {
        at_token: T![@],
        left_paren_token: T!["("],
        items: [CompilerAttribItem],
        right_paren_token: T![")"],
    },
    COMPILER_ATTRIB_ITEM = CompilerAttribItem {
        lhs: Expr,
        eq_token: T![=],
        rhs: Expr,
    },
    NAME = Name {
        ident: T![ident],

        pub fn text(&self) -> Option<Ustr> {
            self.ident().map(|t| Ustr::from(t.text()))
        }
    },

    LUA_BLOCK_EXPR = LuaBlockExpr {
        stmts: [LuaStmt],
    },
    LUA_RETURN_STMT = LuaReturnStmt {

    },
    LUA_WHILE_STMT = LuaWhileStmt {

    },
    LUA_IF_STMT = LuaIfStmt {

    },
    LUA_BREAK_STMT = LuaBreakStmt {},
    LUA_ASSIGN_STMT = LuaAssignStmt {},
    LUA_CONTINUE_STMT = LuaContinueStmt {},
    LUA_FOR_STMT = LuaForStmt {},
    LUA_REPEAT_STMT = LuaRepeatStmt{},
    LUA_FUNCTION_STMT = LuaFunctionStmt{},
    LUA_BLOCK_STMT = LuaBlockStmt {},
    LUA_LOCAL_STMT = LuaLocalStmt {},
}

enums! {
    Item {
        FnItem,
        ModItem,
        StructItem,
    },
    Stmt {
        LetStmt,
        ExprStmt,
    },
    Expr {
        FieldExpr,
        MethodExpr,
        RecordExpr,
        UnitExpr,
        PathExpr,
        BinaryExpr,
        UnaryExpr,
        BlockExpr,
        IndexExpr,
        CallExpr,
        ParenExpr,
        ReturnExpr,
        LitExpr,
        TryExpr,
        IfExpr,
    },
    //TODO: finish patterns
    Pattern {
        NamePattern,
    },
    TypeExpr {
        PathType,
        NilableType,
        LitType,
        AnyType,
        UnitType,
        FnType,
    },

    LuaStmt {
        LuaReturnStmt,
        LuaBreakStmt,
        LuaWhileStmt,
        LuaIfStmt,
        LuaAssignStmt,
        LuaContinueStmt,
        LuaForStmt,
        LuaRepeatStmt,
        LuaBlockStmt,
        LuaFunctionStmt,
        LuaLocalStmt
    }
}

#[cfg(test)]
mod test {
    use rowan::ast::AstNode;

    use crate::parsing::{
        ast::{
            ClosureExpr, FnItem, HasCompilerAttribs, IfExpr, LuaBlockExpr, SyntaxNode, SyntaxToken,
        },
        parser::Lang,
    };

    trait AstTest {
        fn should_eq(&self, expect: &str);
    }

    impl AstTest for SyntaxNode {
        #[track_caller]
        fn should_eq(&self, expect: &str) {
            assert_eq!(self.to_string().trim(), expect);
        }
    }

    impl AstTest for SyntaxToken {
        #[track_caller]
        fn should_eq(&self, expect: &str) {
            assert_eq!(self.to_string(), expect);
        }
    }

    #[track_caller]
    fn parse<N: AstNode<Language = Lang>>(src: &str) -> N {
        let parse = crate::parsing::parser::parse(src);
        assert_eq!(parse.1, vec![]);
        SyntaxNode::new_root(parse.0)
            .descendants()
            .find_map(N::cast)
            .unwrap()
    }

    #[test]
    fn if_expr() {
        let expr = parse::<IfExpr>(
            "fn main() {
            if true {1} else {2}
        }",
        );
        assert!(expr.if_token().is_some());
        assert!(expr.else_token().is_some());
        expr.if_branch().unwrap().syntax().should_eq("{1}");
        expr.if_condition().unwrap().syntax().should_eq("true");
        expr.else_branch().unwrap().syntax().should_eq("{2}");
    }

    #[test]
    fn else_if_expr() {
        let expr = parse::<IfExpr>(
            "fn main() {
            if true {1} else if false {2}
        }",
        );
        assert!(expr.if_token().is_some());
        assert!(expr.else_token().is_some());
        expr.if_branch().unwrap().syntax().should_eq("{1}");
        expr.if_condition().unwrap().syntax().should_eq("true");
        assert!(expr.else_branch().is_none());
        expr.else_if_expr()
            .unwrap()
            .syntax()
            .should_eq("if false {2}");
    }

    #[test]
    fn closure() {
        let closure = parse::<ClosureExpr>(
            "fn main() {
                |x: int, y| x+y
            }",
        );
        closure.syntax().should_eq("|x: int, y| x+y");
    }

    #[test]
    fn attribs() {
        let func = parse::<FnItem>("@first fn main() {}");
        func.syntax().should_eq("fn main() {}");
        func.attribs()
            .unwrap()
            .attribs()
            .next()
            .unwrap()
            .syntax()
            .should_eq("@first");
    }

    #[test]
    fn lua_if_stmt() {
        let block = parse::<LuaBlockExpr>(
            "fn main() {
            lua {
                if true then end
            }
        }",
        );
        block
            .stmts()
            .next()
            .unwrap()
            .syntax()
            .should_eq("if true then end");
    }
}
