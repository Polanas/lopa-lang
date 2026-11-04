use la_arena::Arena;

use crate::{
    def::{AnyId, ElemId, ExprId, ItemTypeExprId, PatId, StmtId, TypeExprId},
    parsing::{self, AstNode},
};

#[derive(Debug, Clone, Default, PartialEq, salsa::SalsaValue, Hash, Eq)]
pub struct BodyMap {
    exprs: Arena<parsing::NodeId>,
    pats: Arena<parsing::NodeId>,
    type_exprs: Arena<parsing::NodeId>,
    item_type_exprs: Arena<parsing::NodeId>,
    elems: Arena<parsing::NodeId>,
    stmts: Arena<parsing::NodeId>,
}

impl BodyMap {
    pub(super) fn insert_expr(&mut self, expr: parsing::Expr) -> ExprId {
        ExprId(self.exprs.alloc(expr.id()))
    }

    pub(super) fn insert_type_expr(&mut self, type_expr: parsing::TypeExpr) -> TypeExprId {
        TypeExprId(self.type_exprs.alloc(type_expr.id()))
    }

    pub(super) fn insert_pat(&mut self, pat: parsing::Pattern) -> PatId {
        PatId(self.pats.alloc(pat.id()))
    }

    pub(super) fn insert_stmt(&mut self, stmt: parsing::Stmt) -> StmtId {
        StmtId(self.stmts.alloc(stmt.id()))
    }

    pub(super) fn insert_elem(&mut self, elem: parsing::Elem) -> ElemId {
        ElemId(self.elems.alloc(elem.id()))
    }

    pub(super) fn insert_item_type_expr(
        &mut self,
        type_expr: parsing::ItemTypeExpr,
    ) -> ItemTypeExprId {
        ItemTypeExprId(self.item_type_exprs.alloc(type_expr.id()))
    }
}

impl std::ops::Index<AnyId> for BodyMap {
    type Output = parsing::NodeId;

    fn index(&self, index: AnyId) -> &Self::Output {
        match index {
            AnyId::ItemTypeExpr(item_type_expr_id) => &self[item_type_expr_id],
            AnyId::TypeExpr(type_expr_id) => &self[type_expr_id],
            AnyId::Elem(elem_id) => &self[elem_id],
            AnyId::Pat(pat_id) => &self[pat_id],
            AnyId::Expr(expr_id) => &self[expr_id],
            AnyId::Stmt(stmt_id) => &self[stmt_id],
        }
    }
}

impl std::ops::Index<ExprId> for BodyMap {
    type Output = parsing::NodeId;

    fn index(&self, index: ExprId) -> &Self::Output {
        &self.exprs[index.0]
    }
}

impl std::ops::Index<TypeExprId> for BodyMap {
    type Output = parsing::NodeId;

    fn index(&self, index: TypeExprId) -> &Self::Output {
        &self.type_exprs[index.0]
    }
}

impl std::ops::Index<ItemTypeExprId> for BodyMap {
    type Output = parsing::NodeId;

    fn index(&self, index: ItemTypeExprId) -> &Self::Output {
        &self.item_type_exprs[index.0]
    }
}

impl std::ops::Index<ElemId> for BodyMap {
    type Output = parsing::NodeId;

    fn index(&self, index: ElemId) -> &Self::Output {
        &self.elems[index.0]
    }
}

impl std::ops::Index<PatId> for BodyMap {
    type Output = parsing::NodeId;

    fn index(&self, index: PatId) -> &Self::Output {
        &self.pats[index.0]
    }
}

impl std::ops::Index<StmtId> for BodyMap {
    type Output = parsing::NodeId;

    fn index(&self, index: StmtId) -> &Self::Output {
        &self.stmts[index.0]
    }
}
