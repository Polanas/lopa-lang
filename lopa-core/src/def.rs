mod ast_id_map;
mod lowering;
#[path = "def/use_tree_map.rs"]
mod use_tree_map_mod;

pub mod body_map;
pub mod hir;
pub mod contents_map;
pub mod items_map;
pub mod mir;
pub mod ty;

pub use ast_id_map::*;
pub use contents_map::*;
pub use use_tree_map_mod::*;
pub use items_map::*;

use crate::parsing;
use la_arena::Idx;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ItemTypeExprId(pub Idx<parsing::NodeId>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TypeExprId(pub Idx<parsing::NodeId>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ElemId(pub Idx<parsing::NodeId>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PatId(pub Idx<parsing::NodeId>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExprId(pub Idx<parsing::NodeId>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StmtId(pub Idx<parsing::NodeId>);

#[salsa::interned(no_lifetime, debug)]
pub struct Symbol {
    #[returns(ref)]
    pub value: String,
}

#[salsa::interned(no_lifetime, debug)]
pub struct SymbolList {
    #[returns(ref)]
    pub symbols: Vec<Symbol>,
}
