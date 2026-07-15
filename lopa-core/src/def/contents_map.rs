use la_arena::{Arena, Idx};

use crate::def::{ElemId, ItemTypeExprId, PatId, TypeExprId};
use crate::parsing::{self, AstNode};

#[derive(Debug, Clone, Default, PartialEq, salsa::Update, Hash, Eq)]
pub struct ContentsMap {
    elems: Arena<parsing::NodeId>,
    type_exprs: Arena<parsing::NodeId>,
    item_type_exprs: Arena<parsing::NodeId>,
    pats: Arena<parsing::NodeId>,
}

impl ContentsMap {
    pub(super) fn insert_item_type_expr(
        &mut self,
        type_expr: parsing::ItemTypeExpr,
    ) -> ItemTypeExprId {
        ItemTypeExprId(self.item_type_exprs.alloc(type_expr.id()))
    }

    pub(super) fn insert_type_expr(&mut self, type_expr: parsing::TypeExpr) -> TypeExprId {
        TypeExprId(self.type_exprs.alloc(type_expr.id()))
    }

    pub(super) fn insert_elem(&mut self, elem: parsing::Elem) -> ElemId {
        ElemId(self.elems.alloc(elem.id()))
    }

    pub(super) fn insert_pat(&mut self, pat: parsing::Pattern) -> PatId {
        PatId(self.pats.alloc(pat.id()))
    }
}

impl std::ops::Index<TypeExprId> for ContentsMap {
    type Output = parsing::NodeId;

    fn index(&self, index: TypeExprId) -> &Self::Output {
        &self.type_exprs[index.0]
    }
}

impl std::ops::Index<ItemTypeExprId> for ContentsMap {
    type Output = parsing::NodeId;

    fn index(&self, index: ItemTypeExprId) -> &Self::Output {
        &self.item_type_exprs[index.0]
    }
}

impl std::ops::Index<ElemId> for ContentsMap {
    type Output = parsing::NodeId;

    fn index(&self, index: ElemId) -> &Self::Output {
        &self.elems[index.0]
    }
}

impl std::ops::Index<PatId> for ContentsMap {
    type Output = parsing::NodeId;

    fn index(&self, index: PatId) -> &Self::Output {
        &self.pats[index.0]
    }
}
