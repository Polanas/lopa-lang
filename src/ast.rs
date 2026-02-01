use crate::{
    common::{self, *},
    position::{Span, Spanned},
};
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
    pub body: Vec<Stmt>,
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
pub struct LitString {
    pub value: String,
    pub kind: common::StringKind,
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
pub struct LitUnit {
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(LitUnit);

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
        Unit(LitUnit),
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
    pub cond: Box<Expr>,
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

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone, strum_macros::Display)]
    pub enum Expr {
        Array(ArrayExpr),
        Binary(BinaryExpr),
        Block(BlockExpr),
        Call(CallExpr),
        Closure(ClosureExpr),
        For(ForExpr),
        FieldGet(FieldGetExpr),
        Group(GroupExpr),
        If(IfExpr),
        Index(IndexExpr),
        Lit(LitExpr),
        Loop(LoopExpr),
        MethodCall(MethodCallExpr),
        Path(Path),
        Ident(Ident),
        Tuple(TupleExpr),
        Unary(UnaryExpr),
        While(WhileExpr),
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

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone, strum_macros::Display)]
    pub enum Pat {
        Ident(PatIdent),
        Paren(PatParen),
        Path(Path),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BindingStmt {
    pub pats: Vec<Pat>,
    pub exprs: Option<Vec<Expr>>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(BindingStmt);

// impl Binding {
//     pub fn as_ref(&'_ self) -> BindingRef<'_> {
//         BindingRef {
//             kind: self.kind,
//             idents: &self.idents,
//             values: self.values.as_deref(),
//         }
//     }
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct BindingRef<'a> {
//     pub idents: &'a [WithSpan<Ident>],
//     pub values: Option<&'a [WithSpan<Expr>]>,
// }
#[derive(Debug, PartialEq, Clone)]
pub struct PrimitiveType {
    pub span: Span,
    pub value: Primitive,
    pub id: AstNodeId,
}
impl_combined!(PrimitiveType);

#[derive(Debug, PartialEq, Clone)]
pub enum ReturnType {
    Default,
    Type(Vec<TypeExpr>),
}

impl ReturnType {
    pub fn span(&self) -> Option<Span> {
        match self {
            ReturnType::Default => None,
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
pub struct SelfType {
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(SelfType);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum TypeExpr {
        Array(Box<TypeExpr>),
        BareFn(BareFnType),
        Nilable(Box<TypeExpr>),
        Path(Path),
        SelfType(SelfType),
        Primitive(PrimitiveType),
        Paren(Box<TypeExpr>),
        Tuple(TupleType),
    }
}

impl std::fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeExpr::Array(type_expr) => todo!(),
            TypeExpr::BareFn(bare_fn_type) => todo!(),
            TypeExpr::Nilable(type_expr) => write!(f, "{type_expr}"),
            TypeExpr::Path(path) => todo!(),
            TypeExpr::SelfType(self_type) => write!(f, "Self"),
            TypeExpr::Primitive(primitive_type) => write!(f, "{}", primitive_type.value),
            TypeExpr::Paren(type_expr) => todo!(),
            TypeExpr::Tuple(tuple_type) => {
                todo!()
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
    pub ty: TypeExpr,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(Receiver);

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
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(FnParamTyped);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum FnParam {
        Receiver(Receiver),
        Typed(FnParamTyped),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ItemFn {
    pub name: Ident,
    pub params: Vec<FnParam>,
    pub body: BlockExpr,
    pub output: ReturnType,
    pub variadic: Option<Variadic>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ItemFn);

#[derive(Debug, PartialEq, Clone)]
pub struct ExternFn {
    pub name: Ident,
    pub params: Vec<FnParam>,
    pub output: ReturnType,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ExternFn);

#[derive(Debug, PartialEq, Clone)]
pub struct InlineFn {
    pub name: Ident,
    pub params: Vec<FnParam>,
    pub output: ReturnType,
    pub body: String,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(InlineFn);

#[derive(Debug, PartialEq, Clone)]
pub enum ExternKind {
    Lua,
    C,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ItemExtern {
    pub kind: ExternKind,
    pub defs: Vec<ExternDefinition>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ItemExtern);

#[derive(Debug, PartialEq, Clone)]
pub struct ItemInline {
    pub defs: Vec<InlineFn>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(ItemInline);

#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    pub ty: TypeExpr,
    pub default_value: Option<Expr>,
    pub name: Option<Ident>,
    pub id: AstNodeId,
    pub span: Span,
}
impl_combined!(Field);

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
}
impl_combined!(ItemStruct);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum ImplItem {
        Fn(ItemFn),
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
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(EnumVariant);

#[derive(Debug, PartialEq, Clone)]
pub struct ItemEnum {
    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ItemEnum);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum Item {
        Fn(ItemFn),
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
    pub exprs: Vec<Expr>,
    pub semi: Option<Span>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ExprStmt);

#[derive(Debug, PartialEq, Clone)]
pub struct AssignStmt {
    pub left: Vec<Expr>,
    pub right: Vec<Expr>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(AssignStmt);

#[derive(Debug, PartialEq, Clone)]
pub struct ReturnStmt {
    pub exprs: Option<Vec<Expr>>,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(ReturnStmt);

#[derive(Debug, PartialEq, Clone)]
pub struct EmptyStmt {
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(EmptyStmt);

#[derive(Debug, PartialEq, Clone)]
pub struct BinaryAssignStmt {
    pub left: Vec<Expr>,
    pub right: Vec<Expr>,
    pub op: BinaryAssignOp,
    pub id: AstNodeId,
    pub span: Span,
}

impl_combined!(BinaryAssignStmt);

impl_combined_enum! {
    #[derive(Debug, PartialEq, Clone)]
    pub enum Stmt {
        Expr(ExprStmt),
        Assign(AssignStmt),
        BinaryAssign(BinaryAssignStmt),
        Binding(BindingStmt),
        Return(ReturnStmt),
        Empty(EmptyStmt),
    }
}

// #[derive(Clone, Debug, PartialEq)]
// pub struct AstType {
//     pub kind: TypeKind,
//     pub nilable: bool,
// }
//
// #[derive(Clone, Debug, PartialEq)]
// pub enum Type {
//     Ast(AstType),
//     Checked(types::Type),
// }
//
// impl std::fmt::Display for Type {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Type::Ast(ast) => write!(f, "{}{}", ast.kind, if self.nilable() { "?" } else { "" }),
//             Type::Checked(checked) => write!(f, "{checked}"),
//         }
//     }
// }
//
// impl Type {
//     pub fn nilable(&self) -> bool {
//         match self {
//             Type::Ast(ast) => ast.nilable,
//             Type::Checked(checked) => checked.nilable,
//         }
//     }
// }
//
// impl From<types::TypeKind> for Type {
//     fn from(value: types::TypeKind) -> Self {
//         Self::Checked(value.into())
//     }
// }
//
// impl From<types::Type> for Type {
//     fn from(value: types::Type) -> Self {
//         Self::Checked(value)
//     }
// }
//
// impl Type {
//     pub fn checked(&self) -> Option<&types::Type> {
//         match self {
//             Type::Checked(checked) => Some(checked),
//             _ => None,
//         }
//     }
// }
//
// impl AstType {
//     pub fn non_nilable(kind: TypeKind) -> Self {
//         Self {
//             kind,
//             nilable: false,
//         }
//     }
//
//     pub fn nilable(kind: TypeKind) -> Self {
//         Self {
//             kind,
//             nilable: true,
//         }
//     }
// }

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
pub struct BareFnParam {
    pub kind: common::FnParamKind,
    pub ident: Option<Ident>,
    pub ty: TypeExpr,
    pub span: Span,
    pub id: AstNodeId,
}
impl_combined!(BareFnParam);
