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

            if inner.is_nilable()
                && let Type::Nilable(deep_inner) = std::mem::replace(&mut **inner, Type::Unknown)
            {
                *self = Type::Nilable(deep_inner)
            }
        }
    }

    pub fn int() -> Self {
        Self::Lit(LitKind::Int)
    }

    pub fn float() -> Self {
        Self::Lit(LitKind::Float)
    }

    pub fn bool() -> Self {
        Self::Lit(LitKind::Bool)
    }

    pub fn any() -> Self {
        Self::Any
    }

    pub fn unit() -> Self {
        Self::Unit
    }

    pub fn unknown() -> Self {
        Self::Unknown
    }

    pub fn never() -> Self {
        Self::Never
    }

    pub fn is_number(&self) -> bool {
        self.is_int() || self.is_float()
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Self::Lit(LitKind::Float))
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Self::Lit(LitKind::Int))
    }

    pub fn collapse_nil(&mut self) {
        self.collapse_nil_inner();
        if let Self::Nilable(inner) = self
            && **inner == Self::Lit(LitKind::Nil)
        {
            *self = Self::Lit(LitKind::Nil);
        }
    }

    pub fn collapsed_nil(mut self) -> Self {
        self.collapse_nil_inner();
        if let Self::Nilable(inner) = &self
            && inner.deref() == &Self::Lit(LitKind::Nil)
        {
            self = Self::Lit(LitKind::Nil);
        }
        self
    }
}

impl<'db> Type<'db> {
    pub fn is_nilable(&self) -> bool {
        matches!(self, Self::Nilable(_) | Self::Lit(LitKind::Nil) | Self::Unit)
    }
}
