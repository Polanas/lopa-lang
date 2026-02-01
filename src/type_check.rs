use std::collections::HashMap;

use crate::{ast::AstNodeId, common::Primitive};

pub enum Type {
    Primitive(Primitive)
}

pub struct TypeChecker {
    types_by_ids: HashMap<AstNodeId, Type>
}
