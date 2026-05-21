pub mod infer;

use std::sync::Arc;

use ustr::Ustr;

use crate::common::LitKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unknown,
    Unit,
    Any,
    Nilable(Box<Self>),
    Lit(LitKind),
    Function {
        params: Vec<(Option<Ustr>, Type)>,
        return_type: Arc<Type>,
    },
}

impl Type {
    pub fn nilable(&self) -> bool {
        matches!(self, Self::Nilable(_))
    }
}
