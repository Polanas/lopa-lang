use crate::def::{
    Symbol,
    ty::{self, Type},
};

#[salsa::interned(debug)]
pub struct Path<'db> {
    #[returns(ref)]
    pub segments: Vec<PathSegment<'db>>,
}

#[salsa::interned(debug)]
pub struct PathSegment<'db> {
    pub ident: Symbol,
    pub generics: Vec<Type<'db>>,
}
