use crate::{
    common::{LitKind, Symbol},
    def::hir,
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
        generics: Vec<Type<'db>>,
    },
    Nilable(Type<'db>),
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
