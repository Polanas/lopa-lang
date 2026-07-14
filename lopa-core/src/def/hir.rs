use std::sync::Arc;

use crate::{
    common::{BinaryOpKind, LitKind, UnaryOpKind},
    def::{
        AstId, ContentsMap, ElemId, ExprId, ItemTypeExprId, PatId, StmtId, Symbol, TypeExprId,
        UseTreeId, body_map::BodyMap,
    },
    ide::{self},
    parsing::{self},
};

#[salsa::interned(debug)]
pub struct Expr<'db> {
    pub id: ExprId,
    pub kind: ExprKind<'db>,
}

//TODO: replace most Expr with Option<Expr> (alternative of Expr::Missing). Also same with Pat
#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum ExprKind<'db> {
    Unit,
    Lit(LitKind),
    Path(Path<'db>),
    As {
        expr: Expr<'db>,
        ty: TypeExpr<'db>,
    },
    Is {
        expr: Expr<'db>,
        pat: Pat<'db>,
    },
    IsNot {
        expr: Expr<'db>,
        pat: Pat<'db>,
    },
    SelfExpr,
    Closure {
        params: ClosureParams<'db>,
        body: Expr<'db>,
        output: Option<TypeExpr<'db>>,
    },
    Field {
        name: Symbol,
        expr: Expr<'db>,
    },
    Method {
        expr: Expr<'db>,
        name: Symbol,
        generic_args: GenericArgs<'db>,
        args: Args<'db>,
    },
    Record {
        path: Path<'db>,
        fields: RecordFields<'db>,
    },
    Binary {
        lhs: Expr<'db>,
        rhs: Expr<'db>,
        kind: BinaryOpKind,
    },
    Unary {
        expr: Expr<'db>,
        kind: UnaryOpKind,
    },
    Block {
        stmts: StmtList<'db>,
    },
    Index {
        base: Expr<'db>,
        index: Expr<'db>,
    },
    Call {
        func: Expr<'db>,
        agrs: Args<'db>,
    },
    Paren(Expr<'db>),
    Return(Expr<'db>),
    If {
        cond: Expr<'db>,
        if_branch: Expr<'db>,
        else_branch: Option<Expr<'db>>,
    },
    Tuple {
        exprs: ExprList<'db>,
    },
}

#[salsa::interned(debug)]
pub struct ExprList<'db> {
    #[returns(ref)]
    pub types: Vec<Expr<'db>>,
}

#[salsa::interned(debug)]
pub struct ClosureParams<'db> {
    #[returns(ref)]
    pub params: Vec<ClosureParam<'db>>,
}

#[salsa::interned(debug)]
pub struct ClosureParam<'db> {
    pub pattern: Pat<'db>,
    pub ty: Option<TypeExpr<'db>>,
}

#[salsa::interned(debug)]
pub struct RecordFields<'db> {
    #[returns(ref)]
    pub fields: Vec<RecordField<'db>>,
}

#[salsa::interned(debug)]
pub struct RecordField<'db> {
    pub name: Symbol,
    pub expr: Expr<'db>,
}

#[salsa::interned(debug)]
pub struct Args<'db> {
    #[returns(ref)]
    pub args: Vec<Arg<'db>>,
}

#[salsa::interned(debug)]
pub struct Arg<'db> {
    pub kind: ArgKind<'db>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum ArgKind<'db> {
    Labeled { label: Symbol, value: Expr<'db> },
    NonLabeled { value: Expr<'db> },
}
impl<'db> ArgKind<'db> {
    pub fn value(&self) -> Expr<'db> {
        match self {
            ArgKind::Labeled { value, .. } | ArgKind::NonLabeled { value } => *value,
        }
    }
}

#[salsa::tracked(debug)]
pub struct StmtList<'db> {
    #[returns(ref)]
    pub stmts: Vec<Stmt<'db>>,
}

#[salsa::tracked(debug)]
pub struct Stmt<'db> {
    pub id: StmtId,
    pub kind: StmtKind<'db>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum StmtKind<'db> {
    Let {
        pat: Pat<'db>,
        ty: Option<TypeExpr<'db>>,
        expr: Expr<'db>,
    },
    Expr {
        expr: Expr<'db>,
        semi: Option<()>,
    },
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum PatKind<'db> {
    Path(Path<'db>),
    Name(Symbol),
    Wildcard,
}

#[salsa::interned(debug)]
pub struct Pat<'db> {
    pub id: PatId,
    pub kind: PatKind<'db>,
}

#[derive(salsa::Supertype, PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum Item<'db> {
    Function(Function<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    Use(UseItem<'db>),
    Module(Module<'db>),
    Impl(ImplBlock<'db>),
}

impl<'db> Item<'db> {
    pub fn name(&self, db: &'db dyn salsa::Database) -> Option<Symbol> {
        Some(match self {
            Item::Function(item) => item.name(db),
            Item::Struct(item) => item.name(db),
            Item::Enum(item) => item.name(db),
            Item::Module(item) => Symbol::new(db, "some mod"),
            Item::Use(_) | Item::Impl(_) => return None,
        })
    }
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum InnerItem<'db> {
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    Function(Function<'db>),
}

impl<'db> InnerItem<'db> {
    pub fn name(&self, db: &dyn salsa::Database) -> Symbol {
        match self {
            InnerItem::Struct(item) => item.name(db),
            InnerItem::Enum(item) => item.name(db),
            InnerItem::Function(item) => item.name(db),
        }
    }
}

#[salsa::tracked(debug)]
pub struct ImplBlock<'db> {
    pub file: ide::File,
    pub items: ImplItems<'db>,
    pub id: AstId<parsing::ImplItem<'static>>,
}

#[salsa::tracked(debug)]
pub struct ImplItems<'db> {
    #[returns(ref)]
    pub items: Vec<Function<'db>>,
}

#[derive(salsa::Update, PartialEq, Clone)]
pub struct ImplContents<'db> {
    pub item_map: ContentsMap,
    pub generics: Generics<'db>,
    pub impl_types: ImplTypes<'db>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum ImplTypes<'db> {
    Inherent(TypeExpr<'db>),
    Trait {
        trait_ty: TypeExpr<'db>,
        impl_ty: TypeExpr<'db>,
    },
}

#[salsa::tracked(debug)]
pub struct Function<'db> {
    pub name: Symbol,
    pub file: ide::File,
    pub id: AstId<parsing::FnItem<'static>>,
}

#[derive(salsa::Update, PartialEq, Clone)]
pub struct FunctionContents<'db> {
    pub item_map: ContentsMap,
    pub params: FnParamList<'db>,
    pub generics: Generics<'db>,
    pub output: Option<TypeExpr<'db>>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum ItemFnParam<'db> {
    SelfParam,
    PatParam {
        pat: Option<Pat<'db>>,
        type_expr: Option<TypeExpr<'db>>,
    },
}

#[derive(salsa::Update, PartialEq, Clone)]
pub struct FunctionBody<'db> {
    pub body_map: BodyMap,
    pub body_expr: Expr<'db>,
    pub params: FnBodyParams<'db>,
}

#[derive(salsa::Update, PartialEq, Clone, Hash, Eq, Debug)]
pub struct FieldBody<'db> {
    pub body_map: BodyMap,
    pub body_expr: Expr<'db>,
}

#[salsa::tracked(debug)]
pub struct FnBodyParams<'db> {
    #[returns(ref)]
    pub params: Vec<FnBodyParam<'db>>,
}

#[salsa::interned(debug)]
pub struct FnBodyParam<'db> {
    pub kind: FnBodyParamKind<'db>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum FnBodyParamKind<'db> {
    SelfParam,
    Pat {
        pat: Option<Pat<'db>>,
        expr: Option<Expr<'db>>,
    },
}

#[salsa::tracked(debug)]
pub struct Struct<'db> {
    pub name: Symbol,
    pub file: ide::File,
    #[returns(ref)]
    pub inner_items: Vec<InnerItem<'db>>,
    pub id: AstId<parsing::StructItem<'static>>,
}

#[derive(salsa::Update, PartialEq, Clone)]
pub struct StructContents<'db> {
    pub item_map: ContentsMap,
    pub parent: Option<Path<'db>>,
    pub elems: ElemList<'db>,
}

#[salsa::tracked(debug)]
pub struct ElemList<'db> {
    #[returns(ref)]
    pub elems: Vec<Elem<'db>>,
}

#[salsa::tracked(debug)]
pub struct Generics<'db> {
    #[returns(ref)]
    pub params: Vec<GenericParam<'db>>,
}

#[salsa::interned(debug)]
pub struct GenericParam<'db> {
    pub ident: Symbol,
    #[returns(ref)]
    pub bounds: Vec<TypeExpr<'db>>,
}

#[salsa::interned(debug)]
pub struct Elem<'db> {
    pub id: ElemId,
    pub kind: ElemKind<'db>,
}

#[derive(salsa::Update, PartialEq, Clone, Hash, Debug, Eq)]
pub enum ElemKind<'db> {
    Field(Field<'db>),
    Function(Function<'db>),
}

#[salsa::interned(debug)]
pub struct Field<'db> {
    pub name: Option<Symbol>,
    pub ty: Option<ItemTypeExpr<'db>>,
    pub body: Option<FieldBody<'db>>,
}

#[salsa::tracked(debug)]
pub struct Enum<'db> {
    pub name: Symbol,
    pub file: ide::File,
    #[returns(ref)]
    pub inner_items: Vec<InnerItem<'db>>,
    pub id: AstId<parsing::EnumItem<'static>>,
}

#[derive(salsa::Update, PartialEq, Clone)]
pub struct EnumContents<'db> {
    pub item_map: ContentsMap,
    pub elems: ElemList<'db>,
}

#[salsa::tracked(debug)]
pub struct UseItem<'db> {
    pub file: ide::File,
    pub id: AstId<parsing::UseItem<'static>>,
}

#[salsa::interned(no_lifetime, debug)]
pub struct UseTree {
    pub kind: UseTreeKind,
    pub id: UseTreeId,
}

#[derive(salsa::Update, PartialEq, Clone, Hash, Debug, Eq)]
pub enum UseTreeKind {
    Path { name: Symbol, use_tree: UseTree },
    Super { use_tree: UseTree },
    Root { use_tree: UseTree },
    TreeList(UseTreeList),
    Name(Symbol),
    SelfUse,
    Global,
}

#[salsa::interned(no_lifetime, debug)]
pub struct UseTreeList {
    #[returns(ref)]
    pub items: Vec<UseTree>,
}

#[derive(salsa::Update, PartialEq, Clone, Hash, Debug, Eq)]
pub enum ModuleKind<'db> {
    Root {
        items: Arc<Vec<Item<'db>>>,
    },
    Definition {
        items: Arc<Vec<Item<'db>>>,
        id: AstId<parsing::ModItem<'static>>,
    },
    Declaration {
        id: AstId<parsing::ModItem<'static>>,
    },
}

#[salsa::tracked(debug)]
pub struct Module<'db> {
    pub name: Symbol,
    #[returns(ref)]
    pub kind: ModuleKind<'db>,
    pub root: ide::Root,
}

impl<'db> Module<'db> {
    pub fn id(&self, db: &'db dyn salsa::Database) -> Option<AstId<parsing::ModItem<'static>>> {
        Some(match self.kind(db) {
            ModuleKind::Definition { id, .. } | ModuleKind::Declaration { id } => *id,
            _ => return None,
        })
    }
}

#[salsa::interned(debug)]
pub struct ItemTypeExpr<'db> {
    pub id: ItemTypeExprId,
    pub kind: ItemTypeExprKind<'db>,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum ItemTypeExprKind<'db> {
    TypeExpr(TypeExpr<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
}

#[salsa::interned(debug)]
pub struct TypeExpr<'db> {
    pub id: TypeExprId,
    pub kind: TypeExprKind<'db>,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum TypeExprKind<'db> {
    Any,
    Unit,
    Never,
    SelfTy,
    Tuple(TypeExprList<'db>),
    Lit(LitKind),
    Path(Path<'db>),
    Dyn(Path<'db>),
    Nilable(TypeExpr<'db>),
    Paren(TypeExpr<'db>),
    Fn {
        params: FnTypeParamList<'db>,
        output: Option<TypeExpr<'db>>,
    },
}

#[salsa::interned(debug)]
pub struct TypeExprList<'db> {
    #[returns(ref)]
    pub types: Vec<TypeExpr<'db>>,
}

#[salsa::interned(debug)]
pub struct FnParamList<'db> {
    #[returns(ref)]
    pub params: Vec<FnParam<'db>>,
}

#[salsa::interned(debug)]
pub struct FnParam<'db> {
    pub kind: FnParamKind<'db>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum FnParamKind<'db> {
    SelfParam,
    Pat {
        pat: Option<Pat<'db>>,
        ty: Option<TypeExpr<'db>>,
    },
}

#[salsa::interned(debug)]
pub struct FnTypeParam<'db> {
    pub name: Option<Symbol>,
    pub ty: Option<TypeExpr<'db>>,
}

#[salsa::interned(debug)]
pub struct FnTypeParamList<'db> {
    #[returns(ref)]
    params: Vec<FnTypeParam<'db>>,
}

#[salsa::interned(debug)]
pub struct Path<'db> {
    #[returns(ref)]
    pub segments: Vec<PathSegment<'db>>,
}

#[salsa::interned(debug)]
pub struct PathSegment<'db> {
    pub ident: Symbol,
    pub args: GenericArgs<'db>,
}

#[salsa::interned(debug)]
pub struct GenericArgs<'db> {
    #[returns(ref)]
    pub generic_args: Vec<Option<TypeExpr<'db>>>,
}

// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub struct FnTypeParam {
//     pub name: Symbol,
//     pub ty: TypeExprId,
// }
//
// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub struct Path {
//     pub segments: Vec<PathSegment>,
// }
//
// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub struct PathSegment {
//     pub ident: Symbol,
//     pub generic_args: Vec<TypeExprId>,
// }
