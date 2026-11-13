use itertools::Itertools;

use crate::common::TypeLitKind;
use crate::def::Symbol;
use crate::{common::LitKind, def::hir};

#[salsa::interned(debug)]
pub struct Path<'db> {
    #[returns(deref)]
    pub segments: Vec<PathSegment<'db>>,
}

#[salsa::interned(debug)]
pub struct PathSegment<'db> {
    #[returns(copy)]
    pub ident: Symbol,
    #[returns(copy)]
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
    #[returns(copy)]
    pub kind: TypeKind<'db>,
}

impl<'db> Type<'db> {
    pub fn is_unknown(&self, db: &'db dyn salsa::Database) -> bool {
        match self.kind(db) {
            TypeKind::Unknown => true,
            TypeKind::Nilable(ty) if matches!(ty.kind(db), TypeKind::Unknown) => true,
            _ => false,
        }
    }

    pub fn is_nilable(&self, db: &'db dyn salsa::Database) -> bool {
        matches!(self.kind(db), TypeKind::Nilable(_)) //should Unit also be nilable?
    }
}
// impl TypeChecker {
//     // Expression typing
//     fn infer_expr(&mut self, expr: Expr) -> Type;
//
//     fn check_expr(&mut self, expr: Expr, expected: Type) -> Type;
//
//     // Type relationships
//     fn is_subtype(&self, sub: &Type, sup: &Type) -> bool;
//
//     fn can_coerce(&self, actual: &Type, expected: &Type) -> bool;
//
//     fn join_types(&self, lhs: &Type, rhs: &Type) -> Type;
//
//     // Conditional semantics
//     fn check_condition(&mut self, expr: Expr) -> Type;
//
//     // Flow-sensitive analysis
//     fn refine_type(&self, ty: &Type, condition: Condition) -> Type;
// }

#[salsa::tracked]
impl<'db> Type<'db> {
    #[salsa::tracked(returns(copy))]
    pub fn stringify(self, db: &'db dyn salsa::Database) -> Symbol {
        match self.kind(db) {
            TypeKind::Unknown => Symbol::new(db, "unknown"),
            TypeKind::Any => Symbol::new(db, "any"),
            TypeKind::Unit => Symbol::new(db, "()"),
            TypeKind::Never => Symbol::new(db, "!"),
            TypeKind::Generic(symbol) => symbol,
            TypeKind::Lit(lit_kind) => Symbol::new(
                db,
                match lit_kind {
                    TypeLitKind::Float => "float",
                    TypeLitKind::Int => "int",
                    TypeLitKind::String => "string",
                    TypeLitKind::Bool => "bool",
                },
            ),
            TypeKind::Struct { value, generics } => {
                Symbol::new(db, format!("struct {}", value.name(db).value(db)))
            }
            TypeKind::Enum { value, generics } => {
                Symbol::new(db, format!("enum {}", value.name(db).value(db)))
            }
            TypeKind::Trait { value, generics } => {
                Symbol::new(db, format!("trait {}", value.name(db).value(db)))
            }
            TypeKind::Class { value, generics } => {
                Symbol::new(db, format!("class {}", value.name(db).value(db)))
            }
            TypeKind::Function { value, generics } => {
                // let output = value.contents(db).output.map(|o| o. ).unwrap_or_else(|| Symbol::new(db, ""));
                todo!()
            }
            TypeKind::BareFn(bare_fn) => todo!(),
            TypeKind::Dyn(type_list) => todo!(),
            TypeKind::Tuple(type_list) => todo!(),
            TypeKind::Nilable(_) => todo!(),
        }
    }
}

#[derive(salsa::SalsaValue, Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum TypeKind<'db> {
    Unknown,
    Any,
    Unit,
    Never,
    Generic(Symbol),
    Lit(TypeLitKind),
    Struct {
        value: hir::Struct<'db>,
        generics: TypeList<'db>,
    },
    Enum {
        value: hir::Enum<'db>,
        generics: TypeList<'db>,
    },
    Trait {
        value: hir::Trait<'db>,
        generics: TypeList<'db>,
    },
    Class {
        value: hir::Class<'db>,
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

impl<'db> TypeKind<'db> {
    pub fn nilable(db: &'db dyn salsa::Database, ty: Type<'db>) -> Self {
        match ty.kind(db) {
            kind @ Self::Nilable(_) => kind,
            _ => Self::Nilable(ty),
        }
    }
}

#[salsa::interned(debug)]
pub struct BareFn<'db> {
    #[returns(copy)]
    pub params: BareFnParams<'db>,
    #[returns(copy)]
    pub output: Type<'db>,
}

#[salsa::interned(debug)]
pub struct BareFnParams<'db> {
    #[returns(deref)]
    pub params: Vec<BareFnParam<'db>>,
}

#[salsa::interned(debug)]
pub struct BareFnParam<'db> {
    #[returns(copy)]
    pub name: Option<Symbol>,
    #[returns(copy)]
    pub ty: Type<'db>,
}

#[salsa::interned(debug)]
pub struct TypeList<'db> {
    #[returns(deref)]
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
    #[returns(deref)]
    pub params: Vec<GenericParam<'db>>,
}

impl<'db> Generics<'db> {
    pub fn empty(db: &'db dyn salsa::Database) -> Self {
        Self::new(db, [])
    }

    pub fn param(&self, db: &'db dyn salsa::Database, name: Symbol) -> Option<&GenericParam<'db>> {
        self.params(db).iter().find(|p| p.name(db) == name)
    }
}

#[salsa::interned(debug)]
pub struct GenericParam<'db> {
    #[returns(copy)]
    pub name: Symbol,
    #[returns(copy)]
    pub bounds: TypeList<'db>,
}

#[salsa::interned(debug)]
pub struct FnParam<'db> {
    #[returns(copy)]
    pub name: Option<Symbol>,
    #[returns(copy)]
    pub ty: Type<'db>,
}

#[salsa::interned(debug)]
pub struct FnParams<'db> {
    #[returns(deref)]
    pub params: Vec<FnParam<'db>>,
}

// #[salsa::tracked]
// impl<'db> hir::Function<'db> {
//     #[salsa::tracked]
//     pub fn params(self, db: &'db dyn salsa::Database) -> FnParams<'db> {
//         // let mut params = vec![];
//         todo!()
//     }
// }
