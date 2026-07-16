use std::sync::Arc;

use itertools::Itertools;

use crate::{
    common::{BinaryOpKind, LitKind, UnaryOpKind},
    def::{
        AstId, ContentsMap, ElemId, ExprId, ItemTypeExprId, PatId, StmtId, Symbol, SymbolList,
        TypeExprId, UseTreeId, body_map::BodyMap,
    },
    ide::{self, InFile},
    parsing::{self},
};

#[derive(Debug, Clone, Copy, PartialEq, salsa::SalsaValue, Hash, Eq)]
pub enum IdSource<'db> {
    BodySource(BodyMapSource<'db>),
    ContentsSource(ContentsMapSource<'db>),
}

impl<'db> IdSource<'db> {
    pub fn get_pure(&self, db: &'db dyn salsa::Database) -> IdSourcePure {
        match self {
            IdSource::BodySource(source) => IdSourcePure::BodySource(source.body_map(db)),
            IdSource::ContentsSource(source) => {
                IdSourcePure::ContentsSource(source.contents_map(db))
            }
        }
    }
}

//if you have better ideas for the name of this type, tell me
#[derive(Debug, Clone, PartialEq, salsa::SalsaValue, Hash, Eq)]
pub enum IdSourcePure {
    BodySource(Arc<BodyMap>),
    ContentsSource(Arc<ContentsMap>),
}

#[derive(Debug, Clone, Copy, PartialEq, salsa::SalsaValue, Hash, Eq)]
pub enum ContentsMapSource<'db> {
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    Impl(ImplBlock<'db>),
    Function(Function<'db>),
}

impl<'db> ContentsMapSource<'db> {
    pub fn contents_map(&self, db: &'db dyn salsa::Database) -> Arc<ContentsMap> {
        match self {
            ContentsMapSource::Struct(item) => item.contents(db).item_map.clone(),
            ContentsMapSource::Enum(item) => item.contents(db).item_map.clone(),
            ContentsMapSource::Impl(item) => item.contents(db).item_map.clone(),
            ContentsMapSource::Function(item) => item.contents(db).item_map.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, salsa::SalsaValue, Hash, Eq)]
pub enum BodyMapSource<'db> {
    Function(Function<'db>),
    Field {
        struct_id: StructId,
        field: Field<'db>,
    },
}

impl<'db> BodyMapSource<'db> {
    pub fn body_map(&self, db: &'db dyn salsa::Database) -> Arc<BodyMap> {
        match self {
            BodyMapSource::Function(item) => item.body(db).body_map.clone(),
            BodyMapSource::Field { struct_id, field } => {
                let struct_item = struct_id.file.items_map(db)[*struct_id];
                struct_item.contents(db).field_bodies[field]
                    .body_map
                    .clone()
            }
        }
    }
}

#[salsa::tracked(debug)]
pub struct Expr<'db> {
    #[returns(copy)]
    pub id: ExprId,
    #[returns(copy)]
    pub kind: ExprKind<'db>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
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

#[salsa::tracked(debug)]
pub struct ExprList<'db> {
    #[returns(deref)]
    pub types: Vec<Expr<'db>>,
}

#[salsa::tracked(debug)]
pub struct ClosureParams<'db> {
    #[returns(deref)]
    pub params: Vec<ClosureParam<'db>>,
}

#[salsa::tracked(debug)]
pub struct ClosureParam<'db> {
    #[returns(copy)]
    pub pattern: Pat<'db>,
    #[returns(copy)]
    pub ty: Option<TypeExpr<'db>>,
}

#[salsa::tracked(debug)]
pub struct RecordFields<'db> {
    #[returns(deref)]
    pub fields: Vec<RecordField<'db>>,
}

#[salsa::tracked(debug)]
pub struct RecordField<'db> {
    #[returns(copy)]
    pub name: Symbol,
    #[returns(copy)]
    pub expr: Expr<'db>,
}

#[salsa::tracked(debug)]
pub struct Args<'db> {
    #[returns(deref)]
    pub args: Vec<Arg<'db>>,
}

#[salsa::tracked(debug)]
pub struct Arg<'db> {
    #[returns(copy)]
    pub kind: ArgKind<'db>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
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
    #[returns(deref)]
    pub stmts: Vec<Stmt<'db>>,
}

#[salsa::tracked(debug)]
pub struct Stmt<'db> {
    #[returns(copy)]
    pub id: StmtId,
    #[returns(copy)]
    pub kind: StmtKind<'db>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
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

#[derive(PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
pub enum PatKind<'db> {
    Path(Path<'db>),
    Name(Symbol),
    Wildcard,
}

#[salsa::tracked(debug)]
pub struct Pat<'db> {
    #[returns(copy)]
    pub id: PatId,
    #[returns(copy)]
    pub kind: PatKind<'db>,
}

pub type FunctionId = InFile<AstId<parsing::FnItem<'static>>>;
pub type StructId = InFile<AstId<parsing::StructItem<'static>>>;
pub type EnumId = InFile<AstId<parsing::EnumItem<'static>>>;
pub type UseItemId = InFile<AstId<parsing::UseItem<'static>>>;
pub type ModuleId = InFile<AstId<parsing::ModItem<'static>>>;
pub type ImplBlockId = InFile<AstId<parsing::ImplItem<'static>>>;

#[derive(salsa::Supertype, PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
pub enum Item<'db> {
    Function(Function<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    Use(UseItem<'db>),
    Module(Module<'db>),
    Impl(ImplBlock<'db>),
}

#[salsa::tracked(debug)]
pub struct Items<'db> {
    #[returns(deref)]
    pub items: Vec<Item<'db>>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
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
    #[returns(copy)]
    pub file: ide::File,
    #[tracked]
    #[returns(copy)]
    //TODO: replace these with Vecs directly
    pub items: ImplItems<'db>,
    #[tracked]
    #[returns(copy)]
    pub id: ImplBlockId,
}

#[salsa::tracked(debug)]
pub struct ImplItems<'db> {
    #[returns(deref)]
    pub items: Vec<Function<'db>>,
}

#[derive(Debug, Clone, PartialEq, salsa::SalsaValue, Hash, Eq)]
pub struct ImplContents<'db> {
    pub item_map: Arc<ContentsMap>,
    pub generics: Generics<'db>,
    pub impl_types: ImplTypes<'db>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
pub enum ImplTypes<'db> {
    Inherent(TypeExpr<'db>),
    Trait {
        trait_ty: TypeExpr<'db>,
        impl_ty: TypeExpr<'db>,
    },
}

#[salsa::tracked(debug)]
pub struct Function<'db> {
    #[returns(copy)]
    pub name: Symbol,
    #[returns(copy)]
    pub file: ide::File,
    #[tracked]
    #[returns(copy)]
    pub id: FunctionId,
}

#[derive(Debug, Clone, PartialEq, salsa::SalsaValue, Hash, Eq)]
pub struct FunctionContents<'db> {
    pub item_map: Arc<ContentsMap>,
    pub params: FnParamList<'db>,
    pub generics: Generics<'db>,
    pub output: Option<TypeExpr<'db>>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
pub enum ItemFnParam<'db> {
    SelfParam,
    PatParam {
        pat: Option<Pat<'db>>,
        type_expr: Option<TypeExpr<'db>>,
    },
}

#[derive(salsa::SalsaValue, PartialEq, Clone, Hash, Eq, Debug)]
pub struct FunctionBody<'db> {
    pub body_map: Arc<BodyMap>,
    pub body_expr: Expr<'db>,
    pub params: FnBodyParams<'db>,
}

#[derive(salsa::SalsaValue, PartialEq, Clone, Hash, Eq, Debug)]
pub struct FieldBody<'db> {
    pub body_map: Arc<BodyMap>,
    pub body_expr: Expr<'db>,
}

#[salsa::tracked(debug)]
pub struct FnBodyParams<'db> {
    #[returns(deref)]
    pub params: Vec<FnBodyParam<'db>>,
}

#[salsa::tracked(debug)]
pub struct FnBodyParam<'db> {
    #[returns(copy)]
    pub kind: FnBodyParamKind<'db>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
pub enum FnBodyParamKind<'db> {
    SelfParam,
    Pat {
        pat: Option<Pat<'db>>,
        expr: Option<Expr<'db>>,
    },
}

#[salsa::tracked(debug)]
pub struct Struct<'db> {
    #[returns(copy)]
    pub name: Symbol,
    #[returns(copy)]
    pub file: ide::File,
    #[tracked]
    #[returns(deref)]
    pub inner_items: Vec<InnerItem<'db>>,
    #[tracked]
    #[returns(copy)]
    pub id: StructId,
}

#[derive(Debug, Clone, PartialEq, salsa::SalsaValue, Eq)]
pub struct StructContents<'db> {
    pub item_map: Arc<ContentsMap>,
    pub parent: Option<Path<'db>>,
    pub elems: ElemList<'db>,
    pub field_bodies: indexmap::IndexMap<Field<'db>, Arc<FieldBody<'db>>>,
}

#[salsa::tracked(debug)]
pub struct ElemList<'db> {
    #[returns(deref)]
    pub elems: Vec<Elem<'db>>,
}

#[salsa::tracked(debug)]
pub struct Generics<'db> {
    #[returns(deref)]
    pub params: Vec<GenericParam<'db>>,
}

#[salsa::tracked(debug)]
pub struct GenericParam<'db> {
    #[returns(copy)]
    pub ident: Symbol,
    #[returns(deref)]
    pub bounds: Vec<TypeExpr<'db>>,
}
//TODO: add #[tracked] annotations

#[salsa::tracked(debug)]
pub struct Elem<'db> {
    #[returns(copy)]
    pub id: ElemId,
    #[returns(copy)]
    pub kind: ElemKind<'db>,
}

#[derive(salsa::SalsaValue, PartialEq, Clone, Copy, Hash, Debug, Eq)]
pub enum ElemKind<'db> {
    Field(Field<'db>),
    Function(Function<'db>),
}

#[salsa::tracked(debug)]
pub struct Field<'db> {
    pub name: Option<Symbol>,
    pub ty: Option<ItemTypeExpr<'db>>,
}

#[salsa::tracked(debug)]
pub struct Enum<'db> {
    #[returns(copy)]
    pub name: Symbol,
    #[returns(copy)]
    pub file: ide::File,
    #[returns(deref)]
    #[tracked]
    pub inner_items: Vec<InnerItem<'db>>,
    #[tracked]
    #[returns(copy)]
    pub id: EnumId,
}

#[derive(Debug, Clone, PartialEq, salsa::SalsaValue, Hash, Eq)]
pub struct EnumContents<'db> {
    pub item_map: Arc<ContentsMap>,
    pub elems: ElemList<'db>,
}

#[salsa::tracked(debug)]
pub struct UseItem<'db> {
    #[returns(copy)]
    pub file: ide::File,
    #[returns(copy)]
    pub id: UseItemId,
}

#[salsa::tracked(debug)]
pub struct UseTree<'db> {
    #[returns(copy)]
    pub kind: UseTreeKind<'db>,
    #[returns(copy)]
    pub id: UseTreeId,
}

#[derive(salsa::SalsaValue, PartialEq, Clone, Copy, Hash, Debug, Eq)]
pub enum UseTreeKind<'db> {
    Path {
        name: Symbol,
        use_tree: UseTree<'db>,
    },
    Super {
        use_tree: UseTree<'db>,
    },
    Root {
        use_tree: UseTree<'db>,
    },
    TreeList(UseTreeList<'db>),
    Name(Symbol),
    SelfUse,
    Global,
}

#[salsa::tracked(debug)]
pub struct UseTreeList<'db> {
    #[returns(deref)]
    pub items: Vec<UseTree<'db>>,
}

#[derive(salsa::SalsaValue, PartialEq, Clone, Copy, Hash, Debug, Eq)]
pub enum ModuleKind<'db> {
    Root { items: Items<'db> },
    Definition { items: Items<'db>, id: ModuleId },
    Declaration { id: ModuleId },
}

#[salsa::tracked(debug)]
pub struct Module<'db> {
    #[returns(copy)]
    pub name: Symbol,
    #[returns(copy)]
    pub kind: ModuleKind<'db>,
    #[returns(copy)]
    pub root: ide::Root,
}

impl<'db> Module<'db> {
    pub fn id(&self, db: &'db dyn salsa::Database) -> Option<ModuleId> {
        Some(match self.kind(db) {
            ModuleKind::Definition { id, .. } | ModuleKind::Declaration { id } => id,
            _ => return None,
        })
    }
}

#[salsa::tracked(debug)]
pub struct ItemTypeExpr<'db> {
    #[returns(copy)]
    pub id: ItemTypeExprId,
    #[returns(copy)]
    pub kind: ItemTypeExprKind<'db>,
}

#[derive(salsa::SalsaValue, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ItemTypeExprKind<'db> {
    TypeExpr(TypeExpr<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
}

#[salsa::tracked(debug)]
pub struct TypeExpr<'db> {
    #[returns(copy)]
    pub id: TypeExprId,
    #[returns(copy)]
    pub source: IdSource<'db>,
    #[returns(copy)]
    pub kind: TypeExprKind<'db>,
}

#[derive(salsa::SalsaValue, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum TypeExprKind<'db> {
    Any,
    Unit,
    Never,
    SelfTy,
    Tuple(TypeExprList<'db>),
    Lit(LitKind),
    Path(Path<'db>),
    Dyn(TypeExprList<'db>),
    Nilable(TypeExpr<'db>),
    Paren(TypeExpr<'db>),
    Fn {
        params: FnTypeParamList<'db>,
        output: Option<TypeExpr<'db>>,
    },
}

#[salsa::tracked(debug)]
pub struct TypeExprList<'db> {
    #[returns(deref)]
    pub types: Vec<TypeExpr<'db>>,
}

#[salsa::tracked(debug)]
pub struct FnParamList<'db> {
    #[returns(deref)]
    pub params: Vec<FnParam<'db>>,
}

#[salsa::tracked(debug)]
pub struct FnParam<'db> {
    #[returns(copy)]
    pub kind: FnParamKind<'db>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, salsa::SalsaValue, Hash)]
pub enum FnParamKind<'db> {
    SelfParam,
    Pat {
        pat: Option<Pat<'db>>,
        ty: Option<TypeExpr<'db>>,
    },
}

#[salsa::tracked(debug)]
pub struct FnTypeParam<'db> {
    #[returns(copy)]
    pub name: Option<Symbol>,
    #[returns(copy)]
    pub ty: Option<TypeExpr<'db>>,
}

#[salsa::tracked(debug)]
pub struct FnTypeParamList<'db> {
    #[returns(deref)]
    pub params: Vec<FnTypeParam<'db>>,
}

#[salsa::tracked(debug)]
pub struct Path<'db> {
    #[returns(deref)]
    pub segments: Vec<PathSegment<'db>>,
}

impl<'db> Path<'db> {
    pub fn as_symbol_list(&self, db: &'db dyn salsa::Database) -> SymbolList {
        SymbolList::new(
            db,
            self.segments(db)
                .iter()
                .map(|s| s.name(db))
                .collect_vec()
                .as_slice(),
        )
    }
}

#[salsa::tracked(debug)]
pub struct PathList<'db> {
    #[returns(deref)]
    pub paths: Vec<Path<'db>>,
}

#[salsa::tracked(debug)]
pub struct PathSegment<'db> {
    #[returns(copy)]
    pub name: Symbol,
    #[returns(copy)]
    pub args: GenericArgs<'db>,
}

#[salsa::tracked(debug)]
pub struct GenericArgs<'db> {
    #[returns(deref)]
    pub generic_args: Vec<Option<TypeExpr<'db>>>,
}
