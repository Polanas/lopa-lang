use std::sync::Arc;

use la_arena::{Arena, Idx};

use crate::{
    common::{LitKind, Symbol}, def::AstId, ide::{self, Root}, parsing,
};

// pub type ExprId = Idx<Expr>;

// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub enum Expr {
//     Missing,
//     Unit,
//     Lit(LitKind),
//     Path(Path),
//     As {
//         expr: ExprId,
//         ty: TypeExprId,
//     },
//     Is {
//         expr: ExprId,
//         pat: PatId,
//     },
//     IsNot {
//         expr: ExprId,
//         pat: PatId,
//     },
//     SelfExpr,
//     Closure {
//         params: Vec<ClosureParam>,
//         body: ExprId,
//         output: Option<TypeExprId>,
//     },
//     Field {
//         name: Symbol,
//         expr: ExprId,
//     },
//     Method {
//         expr: ExprId,
//         name: Symbol,
//
//         args: Vec<Arg>,
//     },
//     Record {
//         path: Path,
//         fields: Vec<RecordField>,
//     },
//     Binary {
//         lhs: ExprId,
//         rhs: ExprId,
//         kind: BinaryOpKind,
//     },
//     Unary {
//         expr: ExprId,
//         kind: UnaryOpKind,
//     },
//     Block {
//         stmts: Vec<StmtId>,
//     },
//     Index {
//         base: ExprId,
//         index: ExprId,
//     },
//     Call {
//         func: ExprId,
//         agrs: Vec<Arg>,
//     },
//     Paren(ExprId),
//     Return {
//         expr: ExprId,
//     },
//     If {
//         cond: ExprId,
//         if_branch: ExprId,
//         else_branch: ExprId,
//     },
// }

// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub struct ClosureParam {
//     pattern: PatId,
//     ty: Option<TypeExprId>,
// }
//
// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub struct RecordField {
//     pub name: Symbol,
//     pub expr: ExprId,
// }
//
// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub enum Arg {
//     Labeled { label: Symbol, value: ExprId },
//     NonLabeled { value: ExprId },
// }
//
// impl Arg {
//     pub fn value(&self) -> ExprId {
//         match self {
//             Arg::Labeled { value, .. } | Arg::NonLabeled { value } => *value,
//         }
//     }
// }
//
// pub type PatId = Idx<Pat>;
//
// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub enum Pat {
//     Missing,
//     Path(Path),
//     Name(Symbol),
//     Wildcard,
// }
//

// pub type StmtId = Idx<Stmt>;
//
// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub enum Stmt {
//     Let {
//         pat: PatId,
//         ty: Option<TypeExprId>,
//         expr: ExprId,
//     },
//     Expr {
//         expr: ExprId,
//         semi: Option<()>,
//     },
// }
//

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum ItemPat<'db> {
    Path(Path<'db>),
    Name(Symbol),
    Wildcard,
}
#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum Item<'db> {
    Function(Function<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    Use(UseItem<'db>),
    Module(Module<'db>),
    Impl(ImplBlock<'db>),
}

#[salsa::tracked(debug)]
pub struct ImplBlock<'db> {
    pub impl_types: ImplTypes<'db>,
    pub fn_items: Vec<Function<'db>>,
    pub ast_ptr: AstId<parsing::ImplItem<'static>>,
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
    #[returns(ref)]
    pub params: Vec<ItemFnParam<'db>>,
    pub output: Option<TypeExpr<'db>>,
    pub ast_ptr: AstId<parsing::FnItem<'static>>,
}

#[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
pub enum ItemFnParam<'db> {
    SelfParam,
    PatParam {
        pat: Option<ItemPat<'db>>,
        type_expr: Option<TypeExpr<'db>>,
    },
}

// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub enum FnParam {
//     SelfParam,
//     PatParam {
//         pat: PatId,
//         type_expr: TypeExprId,
//         default_value: Option<ExprId>,
//     },
// }
//
// #[salsa::tracked(debug)]
// pub struct Body<'db> {
//     #[returns(ref)]
//     pub exprs: Arena<Expr>,
//     #[returns(ref)]
//     pub pats: Arena<Pat>,
//     #[returns(ref)]
//     pub type_exprs: Arena<TypeExpr>,
//     #[returns(ref)]
//     pub stmts: Arena<Stmt>,
//     pub body_expr: ExprId,
// }

#[salsa::tracked(debug)]
pub struct Struct<'db> {
    pub name: Symbol,
    pub parent: Option<Path<'db>>,
    pub elems: Vec<Elem<'db>>,
    pub ast_ptr: AstId<parsing::StructItem<'static>>,
}

#[salsa::tracked(debug)]
pub struct Generics<'db> {
    #[returns(ref)]
    pub params: Vec<GenericParam<'db>>,
}

#[salsa::interned(debug)]
pub struct GenericParam<'db> {
    pub ident: Symbol,
    pub bounds: Vec<TypeExpr<'db>>,
}

#[derive(salsa::Update, PartialEq, Clone, Hash, Debug, Eq)]
pub enum Elem<'db> {
    Field(Field<'db>),
    Function(Function<'db>),
}

#[salsa::interned(debug)]
pub struct Field<'db> {
    pub name: Option<Symbol>,
    pub ty: Option<ItemTypeExpr<'db>>,
}

#[salsa::tracked(debug)]
pub struct Enum<'db> {
    pub name: Symbol,
    pub elems: Vec<Elem<'db>>,
    pub ast_ptr: AstId<parsing::EnumItem<'static>>,
}

#[salsa::tracked(debug)]
pub struct UseItem<'db> {
    pub ast_ptr: AstId<parsing::UseItem<'static>>,
}


#[derive(salsa::Update, PartialEq, Clone, Hash, Debug, Eq)]
pub enum ModuleKind<'db> {
    Declaration(AstId<parsing::ModItem<'static>>),
    Definition(Arc<Vec<Item<'db>>>),
}

#[salsa::tracked(debug)]
pub struct Module<'db> {
    pub name: Symbol,
    #[returns(ref)]
    pub kind: ModuleKind<'db>,
    pub file: ide::File,
}

// pub type TypeExprId = Idx<TypeExpr>;
//
// #[derive(PartialEq, Eq, Clone, Debug, salsa::Update, Hash)]
// pub enum TypeExpr {
//     Missing,
//     Any,
//     Unit,
//     Never,
//     SelfTy,
//     Lit(LitKind),
//     Path(Path),
//     Dyn(Path),
//     Nilable(TypeExprId),
//     Paren(TypeExprId),
//     Fn {
//         params: Vec<FnTypeParam>,
//         output: Option<TypeExprId>,
//     },
// }

#[salsa::interned(debug)]
pub struct ItemTypeExpr<'db> {
    pub kind: ItemTypeExprKind<'db>,
}

#[salsa::interned(debug)]
pub struct TypeExpr<'db> {
    pub kind: TypeExprKind<'db>,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum ItemTypeExprKind<'db> {
    TypeExpr(TypeExprKind<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum TypeExprKind<'db> {
    Any,
    Unit,
    Never,
    SelfTy,
    Lit(LitKind),
    Path(Path<'db>),
    Dyn(Path<'db>),
    Nilable(TypeExpr<'db>),
    Paren(TypeExpr<'db>),
    Fn {
        params: ItemFnTypeParamList<'db>,
        output: Option<TypeExpr<'db>>,
    },
}

#[salsa::interned(debug)]
pub struct FnTypeParam<'db> {
    pub name: Symbol,
    pub ty: TypeExpr<'db>,
}

#[salsa::interned(debug)]
pub struct ItemFnTypeParamList<'db> {
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
    #[returns(ref)]
    pub generic_args: Vec<TypeExpr<'db>>,
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
