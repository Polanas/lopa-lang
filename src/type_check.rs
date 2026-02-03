use std::collections::HashMap;

use crate::{ast::AstNodeId, common::Primitive, position};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    derive_more::Add,
    derive_more::From,
    derive_more::AddAssign,
)]
pub struct TypeId(pub usize);

#[derive(Debug, Clone)]
pub enum Type {
    Primitive(Primitive),
    Nilable(Box<Type>),
    Struct(TypeId),
    Fn(TypeId),
    Array(Box<Type>),
    Block(Vec<Type>),
    Blank,
}

impl Type {
    pub fn is_nilable(&self) -> bool {
        matches!(self, Self::Nilable(_))
    }

    pub fn unwrap_nil(self) -> Self {
        match self {
            Self::Nilable(inner) => *inner,
            other => other,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Field {
    name: String,
    ty: Type,
    default_value: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Option<HashMap<String, Field>>,
}

#[derive(Debug, Clone)]
pub struct Fn {}

#[derive(Debug, Clone)]
pub enum ComplexType {
    Struct(Struct),
    Fn(Fn),
}

pub struct Context<'a> {
    types_by_ids: HashMap<AstNodeId, Type>,
    types: HashMap<TypeId, ComplexType>,
    diagnostics: Vec<position::Diagnostic>,
    source: Option<&'a str>,
}

impl<'a> Context<'a> {
    pub fn new() -> Self {
        Self {
            types_by_ids: Default::default(),
            diagnostics: Default::default(),
            source: Default::default(),
            types: Default::default(),
        }
    }

    pub fn set_source(&mut self, source: &'a str) {
        self.source = Some(source);
    }

    pub fn source(&self) -> &'a str {
        self.source.as_ref().unwrap()
    }
}

impl<'a> Default for Context<'a> {
    fn default() -> Self {
        Self::new()
    }
}
