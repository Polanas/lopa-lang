pub mod infer;

use std::{ops::Deref, sync::Arc};

use ustr::Ustr;

use crate::{common::LitKind, def::ir};

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update, Hash)]
pub struct Param<'db> {
    pub name: Option<Ustr>,
    pub ty: Type<'db>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update, Hash)]
pub struct BareFn<'db> {
    pub params: Vec<Param<'db>>,
    pub return_type: Box<Type<'db>>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update, Hash)]
pub enum Type<'db> {
    Unknown,
    Never,
    Unit,
    Any,
    Nilable(Box<Type<'db>>),
    Lit(LitKind),
    BareFn(BareFn<'db>),
    Function(ir::Function<'db>),
    Struct(ir::Struct<'db>),
}

impl<'db> Type<'db> {
    fn collapse_nil_inner(&mut self) {
        if let Type::Nilable(inner) = self {
            inner.collapse_nil();

            if inner.nilable()
                && let Type::Nilable(deep_inner) = std::mem::replace(&mut **inner, Type::Unknown)
            {
                *self = Type::Nilable(deep_inner)
            }
        }
    }
    pub fn collapse_nil(&mut self) {
        self.collapse_nil_inner();
        if let Type::Nilable(inner) = self
            && **inner == Type::Lit(LitKind::Nil)
        {
            *self = Type::Lit(LitKind::Nil);
        }
    }

    pub fn collapsed_nil(mut self) -> Self {
        self.collapse_nil_inner();
        if let Type::Nilable(inner) = &self
            && inner.deref() == &Type::Lit(LitKind::Nil)
        {
            self = Type::Lit(LitKind::Nil);
        }
        self
    }
}

impl<'db> Type<'db> {
    pub fn nilable(&self) -> bool {
        matches!(self, Self::Nilable(_))
    }
}
