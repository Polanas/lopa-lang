mod ast_id_map;
mod body_map;
mod items_map;
mod lowering;
#[path = "def/use_tree_map.rs"]
mod use_tree_map_mod;

pub mod hir;
pub mod mir;

use itertools::Itertools;

pub use ast_id_map::*;
pub use body_map::*;
pub use items_map::*;
pub use use_tree_map_mod::*;

use la_arena::Idx;

use crate::parsing;

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
    #[returns(deref)]
    pub value: String,
}

#[salsa::interned(no_lifetime, debug)]
pub struct SymbolList {
    #[returns(deref)]
    pub symbols: Vec<Symbol>,
}

impl SymbolList {
    pub fn to_symbol(self, db: &dyn salsa::Database) -> Symbol {
        Symbol::new(db, self.symbols(db).iter().map(|s| s.value(db)).join("::"))
    }

    pub fn push(self, db: &dyn salsa::Database, symbol: Symbol) -> Self {
        let mut symbols = self.symbols(db).to_vec();
        symbols.push(symbol);
        Self::new(db, symbols)
    }
}
