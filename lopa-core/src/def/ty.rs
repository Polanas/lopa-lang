use crate::{
    common::LitKind,
    def::{Symbol, hir},
};

#[salsa::interned(debug)]
pub struct Type<'db> {
    pub kind: TypeKind<'db>,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum TypeKind<'db> {
    Unknown,
    Any,
    Unit,
    Never,
    Generic(Symbol),
    Lit(LitKind),
    Struct {
        value: hir::Struct<'db>,
        generics: TypeList<'db>,
    },
    Enum {
        value: hir::Enum<'db>,
        generics: TypeList<'db>,
    },
    Function {
        value: hir::Function<'db>,
        generics: TypeList<'db>,
    },
    BareFn(BareFn<'db>),
    Dyn(DynBounds<'db>),
    Tuple(TypeList<'db>),
    Nilable(Type<'db>),
}

#[salsa::interned(debug)]
pub struct BareFn<'db> {
    pub params: BareFnParams<'db>,
    pub output: Type<'db>,
}

#[salsa::interned(debug)]
pub struct BareFnParams<'db> {
    #[returns(ref)]
    pub params: Vec<BareFnParam<'db>>,
}

#[salsa::interned(debug)]
pub struct BareFnParam<'db> {
    pub name: Option<Symbol>,
    pub ty: Type<'db>,
}

#[salsa::interned(debug)]
pub struct DynBounds<'db> {
    #[returns(ref)]
    pub bounds: Vec<DynBound<'db>>,
}

#[salsa::interned(debug)]
pub struct DynBound<'db> {
    pub struct_item: hir::Struct<'db>,
    pub generics: TypeList<'db>,
}

#[salsa::interned(debug)]
pub struct TypeList<'db> {
    #[returns(ref)]
    pub types: Vec<Type<'db>>,
}

#[salsa::interned(debug)]
pub struct Generics<'db> {
    #[returns(ref)]
    pub params: Vec<GenericParam<'db>>,
}

impl<'db> Generics<'db> {
    pub fn empty(db: &'db dyn salsa::Database) -> Self {
        Self::new(db, [])
    }
}

#[salsa::interned(debug)]
pub struct GenericParam<'db> {
    pub ident: Symbol,
    pub bounds: Vec<Type<'db>>,
}
