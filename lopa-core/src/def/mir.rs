use itertools::Itertools;

use crate::def::Symbol;
use crate::{common::LitKind, def::hir};

#[salsa::interned(debug)]
pub struct Path<'db> {
    #[returns(ref)]
    pub segments: Vec<PathSegment<'db>>,
}

#[salsa::interned(debug)]
pub struct PathSegment<'db> {
    pub ident: Symbol,
    pub generics: TypeList<'db>,
}

// #[salsa::interned(debug)]
// pub struct TypeWithId<'db> {
//     pub ty: Type<'db>,
//     pub id: TypeExprId,
//     pub source: TypeExprSourced,
// }

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
    Dyn(TypeList<'db>),
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
pub struct TypeList<'db> {
    #[returns(ref)]
    pub types: Vec<Type<'db>>,
}

impl<'db> TypeList<'db> {
    pub fn from_generics(db: &'db dyn salsa::Database, generics: Generics<'db>) -> Self {
        if generics.params(db).is_empty() {
            return Self::new(db, []);
        }

        Self::new(
            db,
            generics
                .params(db)
                .iter()
                .map(|p| Type::new(db, TypeKind::Generic(p.name(db))))
                .collect_vec(),
        )
    }
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
    pub name: Symbol,
    pub bounds: TypeList<'db>,
}

#[salsa::interned(debug)]
pub struct FnParam<'db> {
    pub name: Option<Symbol>,
    pub ty: Type<'db>,
}

#[salsa::interned(debug)]
pub struct FnParams<'db> {
    pub params: Vec<FnParam<'db>>,
}

#[salsa::tracked]
impl<'db> hir::Function<'db> {
    #[salsa::tracked]
    pub fn params(self, db: &'db dyn salsa::Database) -> FnParams<'db> {
        // let mut params = vec![];
        todo!()
    }
}
