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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub enum AnyId {
    ItemTypeExpr(ItemTypeExprId),
    TypeExpr(TypeExprId),
    Member(MemberId),
    Pat(PatId),
    Expr(ExprId),
    Stmt(StmtId),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub struct ItemTypeExprId(pub Idx<parsing::NodeId>);

impl ItemTypeExprId {
    pub fn into_any(self) -> AnyId {
        AnyId::ItemTypeExpr(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub struct TypeExprId(pub Idx<parsing::NodeId>);

impl TypeExprId {
    pub fn into_any(self) -> AnyId {
        AnyId::TypeExpr(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub struct MemberId(pub Idx<parsing::NodeId>);

impl MemberId {
    pub fn into_any(self) -> AnyId {
        AnyId::Member(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub struct PatId(pub Idx<parsing::NodeId>);

impl PatId {
    pub fn into_any(self) -> AnyId {
        AnyId::Pat(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub struct ExprId(pub Idx<parsing::NodeId>);

impl ExprId {
    pub fn into_any(self) -> AnyId {
        AnyId::Expr(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub struct StmtId(pub Idx<parsing::NodeId>);

impl StmtId {
    pub fn into_any(self) -> AnyId {
        AnyId::Stmt(self)
    }
}

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
