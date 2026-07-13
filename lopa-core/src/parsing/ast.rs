use std::marker::PhantomData;

use crate::{
    common::{BinaryOpKind, LitKind, LuaBinaryOpKind, UnaryOpKind},
    parsing::{Children, Node, NodeId, Syntax},
};

trait NodeWrapper {
    const KIND: u16;
}

pub trait NodeExt<'a> {
    fn kind(&self) -> Syntax;
    fn text(&self, source: &'a str) -> &'a str;
}

impl<'a> NodeExt<'a> for Node<'a> {
    fn kind(&self) -> Syntax {
        unsafe { std::mem::transmute::<u16, Syntax>(self.value()) }
    }

    fn text(&self, source: &'a str) -> &'a str {
        &source[self.range()]
    }
}

pub struct AstChildren<'a, N: AstNode<'a>> {
    inner: Children<'a>,
    phantom: PhantomData<N>,
}

impl<'a, N: AstNode<'a>> AstChildren<'a, N> {
    fn new(node: Node<'a>) -> Self {
        Self {
            inner: node.children(),
            phantom: PhantomData,
        }
    }
}

impl<'a, N: AstNode<'a>> Iterator for AstChildren<'a, N> {
    type Item = N;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find_map(N::cast)
    }
}

pub trait AstNode<'a> {
    fn cast(node: Node<'a>) -> Option<Self>
    where
        Self: Sized;
    fn syntax(&self) -> &Node<'a>;
    fn id(&self) -> NodeId;

    fn child<N: AstNode<'a>>(&self) -> Option<N> {
        self.syntax().children().find_map(N::cast)
    }
    fn children<N: AstNode<'a>>(&self) -> AstChildren<'a, N> {
        AstChildren::new(*self.syntax())
    }
    fn token(&self, kind: Syntax) -> Option<Node<'a>> {
        self.syntax().children().find(|n| n.value() == kind.into())
    }
}

pub trait HasCompilerAttribs<'a>: AstNode<'a> {
    fn attribs(&self) -> Option<CompilerAttribList<'a>> {
        self.syntax().prev().and_then(CompilerAttribList::cast)
    }
}

impl<'a, T: AstNode<'a>> HasCompilerAttribs<'a> for T {}

macro_rules! structs {
    (
      $(
          $kind:ident = $name:ident $([$trait:tt])?
          { $($impl:tt)* } $(,)?
      ),*
    ) => {
        $(
            #[derive(Clone, Copy, Debug)]
            pub struct $name<'a>(pub Node<'a>);

            impl<'a> $name<'a> {
                struct_impl!($($impl)*);
            }

            impl NodeWrapper for $name<'_> {
                const KIND: u16 = Syntax::$kind as u16;
            }

            $(impl $trait for $name{})*

            impl<'a> AstNode<'a> for $name<'a> {
                fn cast(node: Node<'a>) -> Option<Self>
                where
                    Self: Sized {
                    if node.value() == Syntax::$kind.into() {
                        Some(Self(node))
                    } else {
                        None
                    }
                }
                fn syntax(&self) -> &Node<'a> {
                    &self.0
                }
                fn id(&self) -> NodeId {
                    self.0.id()
                }
            }
        )*
    };
}

macro_rules! struct_impl {
    () => {};
    //regular child, any
    ($field:ident: $ast:ident, $($tt:tt)*) => {
        pub fn $field(&self) -> Option<$ast<'a>> {
            self.child()
        }
        struct_impl!($($tt)*);
    };
    //child with an offset
    ($field:ident[$k:tt]: $ast:ident, $($tt:tt)*) => {
        pub fn $field(&self) -> Option<$ast<'a>> { self.children().nth($k) }
        struct_impl!($($tt)*);
    };
    //list of children
    ($field:ident: [$ast:ident], $($tt:tt)*) => {
        pub fn $field(&self) -> AstChildren<'a, $ast<'a>> {
            self.children()
        }
        struct_impl!($($tt)*);
    };
    //token
    ($field:ident: T![$tok:tt], $($tt:tt)*) => {

        pub fn $field(&self) -> Option<Node<'a>> {
            self.token(T![$tok])
        }
        struct_impl!($($tt)*);
    };
    //token with an offset
    ($field:ident[$k:tt]: T![$tok:tt], $($tt:tt)*) => {

        pub fn $field(&self) -> Option<Node<'a>> {
            self.syntax().children().filter(|n| n.is_empty()).nth($k)
        }
        struct_impl!($($tt)*);
    };
    ($($item:item)*) => {
        $($item)*
    }
}

macro_rules! enums_cast {
    (@match $node:ident, { $($acc:tt)* }, enum $variant:ident, $($tail:tt)*) => {
        enums_cast!(
            @match
            $node,
            {
                $($acc)*
                _ if let Some(node) = $variant::cast($node) => Some(Self::$variant(node)),

            },
            $($tail)*
        )
    };
    (@match $node:ident, { $($acc:tt)* }, $variant:ident, $($tail:tt)*) => {
        enums_cast!(
            @match
            $node,
            {
                $($acc)*
                <$variant as NodeWrapper>::KIND => Some(Self::$variant($variant($node))),

            },
            $($tail)*
        )
    };
    (@match $node:ident, {$($acc:tt)*},) => {
        match $node.value() {
            $($acc)*
            _ => None,
        }
    };
    ($node:ident, $($input:tt)*) => {
        enums_cast!(
            @match
            $node,
            { },
            $($input)*,
        )
    };
}

macro_rules! enums {
    (
        $(
            $name:ident {
                $(
                    $variant:ident $(: $enum:ident)?
                ),* $(,)?
            }
        ),* $(,)?
    ) => {
        $(
            #[allow(clippy::enum_variant_names)]
            #[derive(Clone, Copy, Debug)]
            pub enum $name<'a> {
                $($variant($variant<'a>),)*
            }

            impl<'a> AstNode<'a> for $name<'a> {
                fn cast(node: Node<'a>) -> Option<Self>
                where
                    Self: Sized {
                        enums_cast!(node, $($($enum)? $variant),*)
                }
                fn syntax(&self) -> &Node<'a> {
                    match self {
                        $(Self::$variant(e) => e.syntax()),*
                    }
                }
                fn id(&self) -> NodeId {
                    match self {
                        $(Self::$variant(e) => e.id()),*
                    }
                }
            }
        )*
    };
}

structs! {
    MODULE = File {
        items: [Item],
    },
    MOD_ITEM = ModItem {
        mod_token: T![mod],
        name: Name,
        semi: T![;],
        items: [Item],
    },
    IMPL_ITEM = ImplItem {
        impl_token: T![impl],
        generics: Generics,
        first_type[0]: TypeExpr,
        for_token: T![for],
        second_type[1]: TypeExpr,
        functions: [FnItem],
    },
    USE_ITEM = UseItem {
        use_keyword: T![use],
        use_tree: UseTree,
    },
    USE_SUPER_PATH = UseSuperPath {
        super_token: T![super],
        use_tree: UseTree,
    },
    USE_ROOT_PATH = UseRootPath {
        root_token: T![root],
        use_tree: UseTree,
    },
    USE_SELF_NAME = UseSelfName {
        self_token: T![self],
    },
    USE_PATH = UsePath {
        name: Name,
        use_tree: UseTree,
    },
    USE_NAME = UseName {
        name: Name,
    },
    USE_GLOBAL = UseGlobal {
        star_keyword: T![*],
    },
    USE_TREE_LIST = UseTreeList {
        left_brace_token: T!["{"],
        elements: [UseTree],
        right_brace_token: T!["}"],
    },
    ENUM_ITEM = EnumItem {
        enum_token: T![enum],
        name: Name,
        generics: Generics,
        left_brace_token: T!["{"],
        elements: [Elem],
        right_brace_token: T!["}"],
    },
    STRUCT_ITEM = StructItem {
        struct_token: T![struct],
        name: Name,
        generics: Generics,
        parent: Parent,
        left_brace_token: T!["{"],
        elements: [Elem],
        right_brace_token: T!["}"],
    },
    PARENT = Parent {
        colon_token: T![:],
        path: Path,
    },
    FIELD = Field {
        name: Name,
        colon_token: T![:],
        ty: ItemTypeExpr,
        eq_token: T![=],
        default_value: Expr,
    },
    FN_ITEM = FnItem {
        fn_token: T![fn],
        name: Name,
        generics: Generics,
        params: ParamList,
        output: ReturnType,
        semi_token: T![;],
        body: BlockExpr,
    },
    GENERIC_ARGUMENTS = GenericArgs {
        types: [TypeExpr],
    },
    GENERICS = Generics {
        params: [TypeParam],
    },
    TYPE_PARAM = TypeParam {
        name: Name,
        colon_token: T![:],
        bounds: [TypeExpr],
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
        type_expr: TypeExpr,
        eq_token: T![=],
        default_value: Expr,
    },
    EXPR_STMT = ExprStmt {
        expr: Expr,
        semi_token: T![;],
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
    NAME_PAT = NamePattern {
        name: Name,
    },
    PATH_PAT = PathPattern {
        path: Path,
    },
    WILDCARD_PAT = WildcardPattern {
        wildcard_token: T![_],
    },
    TUPLE_TYPE = TupleType {
        types: [TypeExpr],
    },
    NILABLE_TYPE = NilableType {
        ty: TypeExpr,
        mark_token: T![?],
    },
    ANY_TYPE = AnyType {
        path: PathType,
    },
    UNIT_TYPE = UnitType {
        left_paren_token: T!["("],
        right_paren_token: T![")"],
    },
    LIT_TYPE = LitType {
        path: PathType,
        pub fn kind(&self) -> Option<LitKind> {
            let node = self.syntax().first()?;
            Some(match node.kind() {
                Syntax::LIT_TYPE_STRING => LitKind::String,
                Syntax::LIT_TYPE_INT => LitKind::Int,
                Syntax::LIT_TYPE_BOOL => LitKind::Bool,
                Syntax::LIT_TYPE_FLOAT => LitKind::Float,
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
    DYN_TYPE = DynType {
        dyn_keyword: T![dyn],
        path: Path,
    },
    PAREN_TYPE = ParenType {
        left_paren_token: T!["("],
        type_expr: TypeExpr,
        right_paren_token: T![")"],
    },
    SELF_TYPE = SelfType {
        self_token: T![Self],
    },
    TUPLE_EXPR = TupleExpr {
        exprs: [Expr],
    },
    UNARY_EXPR = UnaryExpr {
        expr: Expr,

        pub fn op_token(&self) -> Option<Node<'a>> {
            self.op_details().map(|t| t.0)
        }

        pub fn op_kind(&self) -> Option<UnaryOpKind> {
            self.op_details().map(|t| t.1)
        }

        pub fn op_details(&self) -> Option<(Node<'a>, UnaryOpKind)> {
            self.syntax().children().find_map(|node| {
                let op = match node.kind() {
                    T![-] => UnaryOpKind::Neg,
                    T![!] => UnaryOpKind::Not,
                    _ => return None,
                };
                Some((node, op))
            })
        }
    },
    IS_EXPR = IsExpr {
        expr: Expr,
        is_token: T![is],
        pat: Pattern,
    },
    IS_NOT_EXPR = IsNotExpr {
        expr: Expr,
        is_token: T![is_not],
        pat: Pattern,
    },
    AS_EXPR = AsExpr {
        expr: Expr,
        as_token: T![as],
        type_expr: TypeExpr,
    },
    UNIT_EXPR = UnitExpr {
    },
    PATH_EXPR = PathExpr {
        path: Path,
    },
    BINARY_EXPR = BinaryExpr {
        lhs: Expr,
        rhs[1]: Expr,

        pub fn op_token(&self) -> Option<Node<'a>> {
            self.op_details().map(|t| t.0)
        }

        pub fn op_kind(&self) -> Option<BinaryOpKind> {
            self.op_details().map(|t| t.1)
        }

        pub fn op_details(&self) -> Option<(Node<'a>, BinaryOpKind)> {
            self.syntax().children().find_map(|node| {
                let op = match node.kind() {
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
                Some((node, op))
            })
        }
    },
    CLOSURE_EXPR = ClosureExpr {
        bar_left_token: T![|],
        params: ClosureParamList,
        bar_right_token[1]: T![|],
        return_type: ReturnType,
        body: Expr,
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
    ARG = Arg {
        label: Name,
        colon_token: T![:],
        value: Expr,
    },
    CALL_EXPR = CallExpr {
        func: Expr,
        left_paren_token: T!["("],
        args: [Arg],
        right_paren_token: T![")"],
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
    SELF_EXPR = SelfExpr {
        self_token: T![self],
    },
    FIELD_EXPR = FieldExpr {
        expr: Expr,
        dot_token: T![.],
        name: Name,
    },
    SAFE_FIELD_EXPR = SafeFieldExpr {
        field_expr: FieldExpr,
    },
    METHOD_EXPR = MethodExpr {
        expr: Expr,
        dot_token: T![.],
        name: Name,
        generic_args: GenericArgs,
        left_paren_token: T!["("],
        args: [Arg],
        right_paren_token: T![")"],

    },
    SAFE_METHOD_EXPR = SafeMethodExpr {
        method_expr: MethodExpr,
    },
    RECORD_EXPR = RecordExpr {
        path: Path,
        left_brace_token: T!["{"],
        fields_list: [RecordField],
        right_brace_token: T!["}"],
    },
    RECORD_FIELD = RecordField {
        name: Name,
        colon_token: T![:],
        expr: Expr,
    },
    LIT_EXPR = LitExpr {
        pub fn token(&self) -> Option<Node<'a>> {
            self.syntax().children().find(|n|n.is_empty())
        }

        pub fn kind(&self) -> Option<LitKind> {
            Some(match self.token()?.kind() {
                T![nil] => LitKind::Nil,
                Syntax::INT => LitKind::Int,
                Syntax::FLOAT => LitKind::Float,
                Syntax::STRING => LitKind::String,
                Syntax::TRUE_KW | Syntax::FALSE_KW => LitKind::Bool,
                _ => return None,
            })
        }
    },
    PATH = Path {
        segments: [PathSegment],
    },
    PATH_SEGMENT = PathSegment {
        generic_args: GenericArgs,

        pub fn ident(&self, source: &'a str) -> Option<&'a str> {
            self.0.children().filter(|node| node.is_empty()).filter_map(|node| match node.kind() {
                Syntax::SUPER_KW => Some("super"),
                Syntax::ROOT_KW => Some("root"),
                Syntax::IDENT => Some(&source[node.range()]),
                _ => None,
            }).next()
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

        pub fn text(&self, source: &'a str) -> Option<&'a str> {
            self.ident().map(|t| &source[t.range()])
        }
    },
    LUA_CHUNK_EXPR = LuaChunkExpr {
        lua_keyword: T![lua],
        left_brace_token: T!["{"],
        stmts: [LuaStmt],
        right_brace_token: T!["}"],
    },
    LUA_RETURN_STMT = LuaReturnStmt {
        return_keyword: T![return],
        expr: LuaExprMulti,
        semi: T![;],
    },
    LUA_WHILE_STMT = LuaWhileStmt {
        while_keyword: T![while],
        cond: LuaExpr,
        do_keyword: T![ident],
        body: [LuaStmt],
        end_keyword[1]: T![ident],
    },
    LUA_IF_STMT = LuaIfStmt {
        if_keyword: T![if],
        then_keyword: T![ident],
        stmts: [LuaStmt],
        elseif_blocks: [LuaElseIf],
        else_block: LuaElse,
        end_keyword[1]: T![ident],
    },
    LUA_BREAK_STMT = LuaBreakStmt {
        break_keyword: T![break],
        semi: T![;],
    },
    LUA_STMT_EXPR = LuaAssignStmt {
        lhs: LuaExprMulti,
        eq_token: T![=],
        rhs[1]: LuaExprMulti,
        semi: T![;],
    },
    //TODO: remove this, luajit doesnt support continue
    LUA_CONTINUE_STMT = LuaContinueStmt {
        continue_keyword: T![break],
        semi: T![;],
    },
    LUA_FOR_STMT = LuaForStmt {
        lua_generic_for: LuaGenericFor,
        lua_numeric_for: LuaNumericFor,
    },
    LUA_REPEAT_STMT = LuaRepeatStmt {
        repeat_keyword: T![ident],
        stmts: [Stmt],
        until_keyword[1]: T![ident],
    },
    LUA_FUNCTION_STMT = LuaFunctionStmt {
        function_keyword: T![ident],
        name: LuaName,
        dot_token: T![.],
        colon_token: T![:],
        field_name[1]: LuaName,
        param_list: LuaParamList,
        body: [LuaStmt],
        end_keyword[1]: T![ident],
    },
    LUA_BLOCK_STMT = LuaBlockStmt {
        do_keyword: T![ident],
        stmts: [LuaStmt],
        end_keyword[1]: T![ident],
    },
    LUA_LOCAL_STMT = LuaLocalStmt {
        local_keyword: T![ident],
        names: LuaName,
        eq_token: T![=],
        expr_multi: LuaExprMulti,
        semi: T![;],
    },

    LUA_GENERIC_FOR = LuaGenericFor {
        for_keyword: T![for],
        names: [LuaName],
        in_keyword: T![in],
        expr: Expr,
        do_keyword: T![ident],
        stmts: [LuaStmt],
        end_keyword[1]: T![ident],
    },
    LUA_NUMERIC_FOR = LuaNumericFor {
        for_keyword: T![for],
        name: LuaName,
        eq_token: T![=],
        expr_first: LuaExpr,
        expr_second[1]: LuaExpr,
        expr_third[2]: LuaExpr,
        do_keyword: T![ident],
        stmts: [LuaStmt],
        end_keyword[1]: T![ident],
    },
    LUA_ELSEIF = LuaElseIf {
        elseif_keywod: T![ident],
        cond: LuaExpr,
        then_keywod[1]: T![ident],
        stmts: [LuaStmt],
    },
    LUA_ELSE = LuaElse {
        else_keyword: T![else],
        stmts: [LuaStmt],
    },
    LUA_ARG_LIST = LuaArgList {
        left_paren_token: T!["("],
        args: [LuaArg],
        right_paren_token: T![")"],
    },
    LUA_ARG = LuaArg {
        expr: LuaExpr,
    },
    LUA_PARAM_LIST = LuaParamList {
        left_paren_token: T!["("],
        params: [LuaParam],
        right_paren_token: T![")"],
    },
    LUA_PARAM = LuaParam {
        name: LuaName,
    },

    LUA_MULTI_EXPR = LuaExprMulti {
        exprs: [LuaExpr],
    },
    LUA_LIT_EXPR = LuaLitExpr {
        pub fn token(&self) -> Option<Node<'a>> {
            self.syntax().children().find(|node| node.is_empty())
        }

        pub fn kind(&self) -> Option<LitKind> {
            Some(match self.token()?.kind() {
                T![nil] => LitKind::Nil,
                Syntax::INT => LitKind::Int,
                Syntax::FLOAT => LitKind::Float,
                Syntax::STRING | Syntax::SINGLE_STRING | Syntax::BRACKET_STRING => LitKind::String,
                Syntax::TRUE_KW | Syntax::FALSE_KW => LitKind::Bool,
                _ => return None,
            })
        }
    },
    LUA_INDEX_EXPR = LuaIndexExpr {
        base: LuaExpr,
        left_bracket_token: T!["["],
        index[1]: LuaExpr,
        right_bracket_token: T!["]"],
    },
    LUA_CALL_EXPR = LuaCallExpr {
        func: Expr,
        args: LuaArgList,
    },
    LUA_UNARY_EXPR = LuaUnaryExpr {
        expr: LuaExpr,

        pub fn op_token(&self) -> Option<Node<'a>> {
            self.op_details().map(|t| t.0)
        }

        pub fn op_kind(&self) -> Option<UnaryOpKind> {
            self.op_details().map(|t| t.1)
        }

        pub fn op_details(&self) -> Option<(Node<'a>, UnaryOpKind)> {
            self.syntax().children().find_map(|node| {
                let op = match node.kind() {
                    T![-] => UnaryOpKind::Neg,
                    T![not] => UnaryOpKind::Not,
                    _ => return None,
                };
                Some((node, op))
            })
        }
    },
    LUA_BINARY_EXPR = LuaBinaryExpr {
        lhs: LuaExpr,
        rhs[1]: LuaExpr,

        pub fn op_token(&self) -> Option<Node<'a>> {
            self.op_details().map(|t| t.0)
        }

        pub fn op_kind(&self) -> Option<LuaBinaryOpKind> {
            self.op_details().map(|t| t.1)
        }

        pub fn op_details(&self) -> Option<(Node<'a>, LuaBinaryOpKind)> {
            self.syntax().children().find_map(|node| {
                let op = match node.kind() {
                    T![+] => LuaBinaryOpKind::Add,
                    T![*] => LuaBinaryOpKind::Mul,
                    T![/] => LuaBinaryOpKind::Div,
                    T![%] => LuaBinaryOpKind::Rem,
                    T![or] => LuaBinaryOpKind::Or,
                    T![-] => LuaBinaryOpKind::Sub,
                    T![>] => LuaBinaryOpKind::Greater,
                    T![>=] => LuaBinaryOpKind::GreaterEqual,
                    T![<] => LuaBinaryOpKind::Less,
                    T![<=] => LuaBinaryOpKind::LessEqual,
                    T![!=] => LuaBinaryOpKind::NotEqual,
                    T![==] => LuaBinaryOpKind::Equal,
                    T![and] => LuaBinaryOpKind::And,
                    T![..] => LuaBinaryOpKind::Concat,
                    T![^] => LuaBinaryOpKind::Exp,
                    _ => return None,
                };
                Some((node, op))
            })
        }
    },
    LUA_TABLE_EXPR = LuaTableExpr {
        left_brace_token: T!["{"],
        elems: [LuaTableElem],
        right_brace_token: T!["}"],
    },
    LUA_FIELD_ACCESS_EXPR = LuaFieldAccessExpr {
        dot_token: T![.],
        colon_token: T![:],
        name: LuaName,
    },
    LUA_FUNCTION_EXPR = LuaFunctionExpr {
        function_keyword: T![ident],
        param_list: LuaParamList,
        body: [LuaStmt],
        end_keyword[1]: T![ident],
    },

    LUA_ELEM_EXPR = LuaElemExpr {
        expr: LuaExpr,
    },
    LUA_ELEM_ASSIGN = LuaElemAssign {
        name: LuaName,
        eq_token: T![=],
        expr: LuaExpr,
    },
    LUA_ELEM_INDEX_ASSIGN = LuaElemIndexAssign {
        left_bracket_token: T!["["],
        base: Expr,
        right_bracket_token: T!["]"],
        index[1]: Expr,
    },

    LUA_HASH_NAME = LuaHashName {
        hash_token: T![#],
        name: LuaName,
    },
    LUA_NAME = LuaName {
        ident: T![ident],

        pub fn text(&self, source: &'a str) -> Option<&'a str> {
            self.ident().map(|t| &source[t.range()])
        }
    },
}

// #[allow(clippy::enum_variant_names)]
// #[derive(Clone, Copy, Debug)]
// pub enum ItemTypeExpr<'a> {
//     StructItem(StructItem<'a>),
//     EnumItem(EnumItem<'a>),
//     TypeExpr(TypeExpr<'a>),
// }
// impl<'a> AstNode<'a> for ItemTypeExpr<'a> {
//     fn cast(node: Node<'a>) -> Option<Self>
//     where
//         Self: Sized,
//     {
//         match node.value() {
//             <StructItemType as NodeWrapper>::KIND => Some(Self::Struct(StructItemType(node))),
//             <EnumItemType as NodeWrapper>::KIND => Some(Self::Enum(EnumItemType(node))),
//             _ if let Some(node) = TypeExpr::cast(node) => Some(Self::TypeExpr(node)),
//             _ => None,
//         }
//     }
//     fn syntax(&self) -> &Node<'a> {
//         match self {
//             Self::Struct(e) => e.syntax(),
//             Self::Enum(e) => e.syntax(),
//             Self::TypeExpr(e) => e.syntax(),
//         }
//     }
//     fn id(&self) -> NodeId {
//         match self {
//             Self::Struct(e) => e.id(),
//             Self::Enum(e) => e.id(),
//             Self::TypeExpr(e) => e.id(),
//         }
//     }
// }

enums! {
    Item {
        FnItem,
        ModItem,
        ImplItem,
        StructItem,
        EnumItem,
        UseItem,
    },
    Stmt {
        LetStmt,
        ExprStmt,
    },
    Expr {
        AsExpr,
        IsExpr,
        IsNotExpr,
        SelfExpr,
        ClosureExpr,
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
        IfExpr,
        TupleExpr,
    },
    UseTree {
        UseRootPath,
        UseSuperPath,
        UseSelfName,
        UsePath,
        UseName,
        UseGlobal,
        UseTreeList,
    },
    //After modifying, don't forget to also change Syntax::is_elem!!!
    Elem {
        Field,
        FnItem,
    },
    Pattern {
        NamePattern,
        PathPattern,
        WildcardPattern,
    },
    ItemTypeExpr {
        StructItem,
        EnumItem,
        TypeExpr: enum,
    },
    TypeExpr {
        TupleType,
        DynType,
        ParenType,
        PathType,
        NilableType,
        LitType,
        AnyType,
        UnitType,
        FnType,
        SelfType,
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
    },
    LuaExpr {
        LuaLitExpr,
        LuaIndexExpr,
        LuaCallExpr,
        LuaUnaryExpr,
        LuaBinaryExpr,
        LuaTableExpr,
        LuaFieldAccessExpr,
        LuaFunctionExpr,
    },
    LuaTableElem {
        LuaElemAssign,
        LuaElemIndexAssign,
        LuaElemExpr,
    }
}

#[cfg(test)]
mod test {
    use crate::parsing::parser::parse;

    use super::*;

    trait AstTest {
        fn should_eq(&self, source: &str, expect: &str);
    }

    impl<'a> AstTest for Node<'a> {
        #[track_caller]
        fn should_eq(&self, source: &str, expect: &str) {
            assert_eq!(&source[self.range()], expect);
        }
    }

    #[track_caller]
    fn first<'a, N: AstNode<'a>>(tree: &'a syntree::Tree<u16, syntree::FlavorDefault>) -> N {
        tree.walk().find_map(N::cast).unwrap()
    }

    #[test]
    fn safe_field_expr() {
        let source = "fn main() { a?.b }";
        let tree = parse(source).0;
        first::<SafeFieldExpr>(&tree)
            .field_expr()
            .unwrap()
            .syntax()
            .should_eq(source, "a?.b");
    }
}
