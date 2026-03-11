use crate::{
    common::{self, *},
    position::{Span, Spanned},
};
use itertools::Itertools;
use paste::paste;

macro_rules! impl_spanned {
    ($ident:ident) => {
        impl $crate::position::Spanned for $ident {
            fn span(&self) -> $crate::position::Span {
                self.span
            }
        }
    };
}

macro_rules! impl_combined_enum {
    (
        $( #[$meta:meta] )*
        $vis:vis enum $name:ident {
            $($variant:ident $(($ty:ty))?,)*
        }
    ) => {
        $( #[$meta] )*
        $vis enum $name {
            $($variant $(($ty))?,)*
        }

        #[allow(non_snake_case)]
        impl $crate::position::Spanned for $name {
            fn span(&self) -> $crate::position::Span {
                match self {
                    $($name::$variant ( paste!{[<_ $name>]} ) => {
                        paste!{[<_ $name>]}.span()
                    },)*
                }
            }
        }

        #[allow(non_snake_case)]
        impl AstNode for $name {
            fn node_id(&self) -> AstNodeId {
                match self {
                    $($name::$variant (paste!{[<_ $name>]}) => {
                        paste!{[<_ $name>]}.node_id()
                    },)*
                }
            }
        }
    };
}

macro_rules! impl_ast_node {
    ($ident:ident) => {
        impl AstNode for $ident {
            fn node_id(&self) -> AstNodeId {
                self.id
            }
        }
    };
}

macro_rules! impl_combined {
    ($ident:ident) => {
        impl_spanned!($ident);
        impl_ast_node!($ident);
    };
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    derive_more::Add,
    derive_more::From,
    derive_more::AddAssign,
)]
pub struct AstNodeId(pub usize);

pub trait AstNode {
    fn node_id(&self) -> AstNodeId;
}

#[derive(Debug, PartialEq, Clone)]
pub struct Ident {
    pub value: String,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(Ident);

impl Ident {
    pub fn is_self(&self) -> bool {
        self.value == "self"
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub op: BinaryOp,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(BinaryExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct IfExpr {
    pub condition: Box<Expr>,
    pub value: BlockExpr,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(IfExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct UnaryExpr {
    pub expr: Box<Expr>,
    pub op: UnaryOp,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(UnaryExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct BlockExpr {
    pub stmts: Vec<Stmt>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(BlockExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct FnArg {
    pub name: Option<Ident>,
    pub expr: Box<Expr>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct CallExpr {
    pub func: Box<Expr>,
    pub args: Vec<FnArg>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(CallExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct ClosureExpr {
    pub params: Vec<FnParam>,
    pub body: Box<Expr>,
    pub output: ReturnType,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ClosureExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct NamedMember {
    pub value: Ident,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(NamedMember);

#[derive(Debug, PartialEq, Clone)]
pub struct UnnamedMember {
    pub value: usize,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(UnnamedMember);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum Member {
        Named(NamedMember),
        Unnamed(UnnamedMember),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FieldGetExpr {
    pub base: Box<Expr>,
    pub optional: bool,
    pub member: Member,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(FieldGetExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct LitBool {
    pub value: bool,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(LitBool);

#[derive(Debug, PartialEq, Clone)]
pub struct LitInterpolatedString {
    pub exprs: Option<Vec<(usize, Expr)>>,
    pub value: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LitString {
    pub value: String,
    pub kind: common::StringKind,
    pub interpolated: Option<LitInterpolatedString>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(LitString);

#[derive(Debug, PartialEq, Clone)]
pub struct LitNil {
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(LitNil);

#[derive(Debug, PartialEq, Clone)]
pub struct LitInt {
    pub value: i64,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(LitInt);

#[derive(Debug, PartialEq, Clone)]
pub struct LitFloat {
    pub value: f64,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(LitFloat);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum LitExpr {
        Nil(LitNil),
        Int(LitInt),
        Float(LitFloat),
        Bool(LitBool),
        String(LitString),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GroupExpr {
    pub expr: Box<Expr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(GroupExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct MethodCallExpr {
    pub receiver: Box<Expr>,
    pub optional: bool,
    pub method: Ident,
    pub args: Vec<FnArg>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(MethodCallExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct IndexExpr {
    pub index: Box<Expr>,
    pub indexed: Box<Expr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(IndexExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct LoopExpr {
    pub body: BlockExpr,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(LoopExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct WhileExpr {
    pub condition: Box<Expr>,
    pub body: BlockExpr,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(WhileExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct ForExpr {
    pub pat: Pat,
    pub expr: Box<Expr>,
    pub body: BlockExpr,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ForExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct ArrayExpr {
    pub elements: Vec<Expr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ArrayExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct TupleExpr {
    pub exprs: Vec<Expr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(TupleExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct StructExpr {
    pub path: Path,
    pub fields: Vec<FieldValue>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(StructExpr);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone, strum_macros::Display)]
    pub enum Expr {
        Array(ArrayExpr),
        Assign(AssignExpr),
        Binary(BinaryExpr),
        Block(BlockExpr),
        Break(BreakExpr),
        Call(CallExpr),
        Closure(ClosureExpr),
        Continue(ContinueExpr),
        For(ForExpr),
        FieldGet(FieldGetExpr),
        Group(GroupExpr),
        Ident(Ident),
        If(IfExpr),
        Index(IndexExpr),
        Lit(LitExpr),
        Loop(LoopExpr),
        MethodCall(MethodCallExpr),
        Struct(StructExpr),
        Path(Path),
        Return(ReturnExpr),
        Tuple(TupleExpr),
        Unary(UnaryExpr),
        While(WhileExpr),
        Yield(YieldExpr),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PatIdent {
    pub value: Ident,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(PatIdent);

#[derive(Debug, PartialEq, Clone)]
pub struct PatParen {
    pub pat: Box<Pat>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(PatParen);

#[derive(Debug, PartialEq, Clone)]
pub struct PatTuple {
    pub pats: Vec<Pat>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(PatTuple);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone, strum_macros::Display)]
    pub enum Pat {
        Ident(PatIdent),
        Tuple(PatTuple),
        Paren(PatParen),
        Path(Path),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BindingStmt {
    pub pat: Pat,
    pub ty: Option<TypeExpr>,
    pub expr: Option<Expr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(BindingStmt);

#[derive(Debug, PartialEq, Clone)]
pub struct PrimitiveType {
    pub span: Span,
    pub value: Primitive,
    pub id: AstNodeId,
}
impl_combined!(PrimitiveType);

#[derive(Debug, PartialEq, Clone)]
pub enum ReturnType {
    None,
    Type(Vec<TypeExpr>),
}

impl ReturnType {
    pub fn span(&self) -> Option<Span> {
        match self {
            ReturnType::None => None,
            ReturnType::Type(type_exprs) => Some(
                type_exprs
                    .iter()
                    .fold(type_exprs[0].span(), |span, expr| span.union(expr.span())),
            ),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BareFnType {
    pub params: Vec<BareFnParam>,
    pub variadic: Option<BareVariadic>,
    pub output: ReturnType,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(BareFnType);

#[derive(Debug, PartialEq, Clone)]
pub struct PathSegment {
    pub ident: Ident,
    pub span: Span,
}
impl_spanned!(PathSegment);

#[derive(Debug, PartialEq, Clone)]
pub struct Path {
    pub segments: Vec<PathSegment>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(Path);

#[derive(Debug, PartialEq, Clone)]
pub struct TupleType {
    pub types: Vec<TypeExpr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(TupleType);

#[derive(Debug, PartialEq, Clone)]
pub struct UnionType {
    pub left: Box<TypeExpr>,
    pub right: Box<TypeExpr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(UnionType);

#[derive(Debug, PartialEq, Clone)]
pub struct ParenType {
    pub ty: Box<TypeExpr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ParenType);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum TypeExpr {
        Array(Box<TypeExpr>),
        BareFn(BareFnType),
        Nilable(Box<TypeExpr>),
        Path(Path),
        Receiver(Receiver),
        Primitive(PrimitiveType),
        Paren(ParenType),
        Tuple(TupleType),
        Union(UnionType),
    }
}

impl std::fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeExpr::Array(type_expr) => write!(f, "[{type_expr}]"),
            TypeExpr::BareFn(bare_fn_type) => {
                let params = bare_fn_type
                    .params
                    .iter()
                    .map(|p| match p {
                        BareFnParam::Receiver(_) => String::from("self"),
                        BareFnParam::Typed(typed) => {
                            if let Some(name) = &typed.ident {
                                format!("{}: {}", &name.value, typed.ty)
                            } else {
                                format!("{}", typed.ty)
                            }
                        }
                    })
                    .join(", ");
                let output = match &bare_fn_type.output {
                    ReturnType::None => None,
                    ReturnType::Type(type_exprs) => Some(type_exprs.iter().join(", ")),
                };
                let returns_none = match &bare_fn_type.output {
                    ReturnType::None => true,
                    ReturnType::Type(type_exprs) => matches!(
                        type_exprs.as_slice(),
                        [TypeExpr::Primitive(PrimitiveType {
                            value: Primitive::Nil,
                            ..
                        })]
                    ),
                };
                if let Some(output) = output
                    && !returns_none
                {
                    write!(f, "fn({params}) -> {output}")
                } else {
                    write!(f, "fn({params})")
                }
            }
            TypeExpr::Nilable(type_expr) => write!(f, "{type_expr}?"),
            TypeExpr::Path(path) => write!(
                f,
                "{}",
                path.segments.iter().map(|s| &s.ident.value).join("::")
            ),
            TypeExpr::Receiver(_) => write!(f, "Self"),
            TypeExpr::Primitive(primitive_type) => write!(f, "{}", primitive_type.value),
            TypeExpr::Paren(type_expr) => write!(f, "{}", type_expr.ty),
            TypeExpr::Union(union_type) => write!(f, "{} | {}", union_type.left, union_type.right),
            TypeExpr::Tuple(_tuple_type) => {
                unimplemented!()
            }
        }
    }
}

impl TypeExpr {
    pub fn nilable(&self) -> bool {
        matches!(self, Self::Nilable(_))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Receiver {
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(Receiver);

#[derive(Debug, PartialEq, Clone)]
pub struct FnParamReceiver {
    pub attribs: Vec<Attrib>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(FnParamReceiver);

#[derive(Debug, PartialEq, Clone)]
pub struct PatType {
    pub pat: Box<Pat>,
    pub ty: Box<TypeExpr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(PatType);

#[derive(Debug, PartialEq, Clone)]
pub struct FnParamTyped {
    pub pat_type: PatType,
    pub default_value: Option<Expr>,
    pub attribs: Vec<Attrib>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(FnParamTyped);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum FnParam {
        Receiver(Receiver),
        Typed(Box<FnParamTyped>),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FnType {
    Sync,
    Coroutine,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ItemFn {
    pub fn_type: FnType,
    pub name: Ident,
    pub params: Vec<FnParam>,
    pub body: BlockExpr,
    pub output: ReturnType,
    pub variadic: Option<Variadic>,
    pub attribs: Vec<Attrib>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ItemFn);

#[derive(Debug, PartialEq, Clone)]
pub struct ItemStatic {
    pub ident: Ident,
    pub ty: Option<TypeExpr>,
    pub attribs: Vec<Attrib>,
    pub expr: Expr,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ItemStatic);

#[derive(Debug, PartialEq, Clone)]
pub struct ExternFn {
    pub name: Ident,
    pub params: Vec<FnParam>,
    pub output: ReturnType,
    pub variadic: Option<Variadic>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ExternFn);

#[derive(Debug, PartialEq, Clone)]
pub struct InlineFn {
    pub name: Ident,
    pub params: Vec<FnParam>,
    pub output: ReturnType,
    pub body: LitString,
    pub id: AstNodeId,
    pub variadic: Option<Variadic>,
    pub span: Span,
}
impl_combined!(InlineFn);

#[derive(Debug, PartialEq, Clone)]
pub struct OperatorAttrib {
    pub op: BinaryOp,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(OperatorAttrib);

#[derive(Debug, PartialEq, Clone)]
pub struct ItemAttrib {
    pub expr: CallExpr,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ItemAttrib);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum Attrib {
        Operator(OperatorAttrib),
        Item(ItemAttrib),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExternKind {
    Lua,
    C,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ItemExtern {
    pub kind: ExternKind,
    pub defs: Vec<ExternDefinition>,
    pub attribs: Vec<Attrib>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ItemExtern);

#[derive(Debug, PartialEq, Clone)]
pub struct ItemInline {
    pub defs: Vec<InlineFn>,
    pub attribs: Vec<Attrib>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ItemInline);

#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    pub ty: TypeExpr,
    pub default_value: Option<Expr>,
    pub name: Option<Ident>,
    pub attribs: Vec<Attrib>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(Field);

#[derive(Debug, PartialEq, Clone)]
pub struct FieldValue {
    pub name: Ident,
    pub expr: Box<Expr>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(FieldValue);

#[derive(Debug, PartialEq, Clone)]
pub struct FieldsNamed {
    pub fields: Vec<Field>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(FieldsNamed);

#[derive(Debug, PartialEq, Clone)]
pub struct FieldsUnnamed {
    pub fields: Vec<Field>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(FieldsUnnamed);

#[derive(Debug, PartialEq, Clone)]
pub enum Fields {
    Unit,
    Named(FieldsNamed),
    Unnamed(FieldsUnnamed),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ItemStruct {
    pub name: Ident,
    pub kind: StructKind,
    pub fields: Fields,
    pub span: Span,
    pub id: AstNodeId,
    pub attribs: Vec<Attrib>,
}
impl_combined!(ItemStruct);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum ImplItem {
        Fn(ItemFn),
        Static(ItemStatic),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ItemImpl {
    pub target: TypeExpr,
    pub items: Vec<ImplItem>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ItemImpl);

#[derive(Debug, PartialEq, Clone)]
pub struct EnumVariant {
    pub name: Ident,
    pub fields: Fields,
    pub discriminant: Option<Expr>,
    pub attribs: Vec<Attrib>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(EnumVariant);

#[derive(Debug, PartialEq, Clone)]
pub struct ItemEnum {
    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    pub attribs: Vec<Attrib>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ItemEnum);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum Item {
        Fn(ItemFn),
        Static(ItemStatic),
        Extern(ItemExtern),
        Inline(ItemInline),
        Struct(ItemStruct),
        Enum(ItemEnum),
        Impl(ItemImpl),
    }
}

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum ExternDefinition {
        Fn(ExternFn),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: Expr,
    pub semi: Option<Span>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ExprStmt);

#[derive(Debug, PartialEq, Clone)]
pub struct AssignExpr {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(AssignExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct ReturnExpr {
    pub expr: Option<Box<Expr>>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ReturnExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct ContinueExpr {
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ContinueExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct BreakExpr {
    pub span: Span,
    pub expr: Option<Box<Expr>>,
    pub id: AstNodeId,
}
impl_combined!(BreakExpr);

#[derive(Debug, PartialEq, Clone)]
pub struct YieldExpr {
    pub expr: Option<Box<Expr>>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(YieldExpr);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum Stmt {
        Expr(ExprStmt),
        Binding(BindingStmt),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Variadic {
    pub ident: Option<Ident>,
    pub ty: Box<TypeExpr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(Variadic);

#[derive(Clone, Debug, PartialEq)]
pub struct BareVariadic {
    pub ident: Option<Ident>,
    pub ty: Box<TypeExpr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(BareVariadic);

#[derive(Clone, Debug, PartialEq)]
pub struct BareFnParamTyped {
    pub ident: Option<Ident>,
    pub ty: TypeExpr,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(BareFnParamTyped);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum BareFnParam {
        Receiver(Receiver),
        Typed(BareFnParamTyped),
    }
}
