use la_arena::Arena;

use crate::{
    def::{HirNodeId, MemberId, ExprId, ItemTypeExprId, PatId, StmtId, TypeExprId},
    parsing::{self, AstNode},
};

#[derive(Debug, Clone, Default, PartialEq, salsa::SalsaValue, Hash, Eq)]
pub struct BodyMap {
    exprs: Arena<parsing::NodeId>,
    pats: Arena<parsing::NodeId>,
    type_exprs: Arena<parsing::NodeId>,
    item_type_exprs: Arena<parsing::NodeId>,
    members: Arena<parsing::NodeId>,
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

    pub(super) fn insert_member(&mut self, member: parsing::Member) -> MemberId {
        MemberId(self.members.alloc(member.id()))
    }

    pub(super) fn insert_item_type_expr(
        &mut self,
        type_expr: parsing::ItemTypeExpr,
    ) -> ItemTypeExprId {
        ItemTypeExprId(self.item_type_exprs.alloc(type_expr.id()))
    }
}

impl std::ops::Index<HirNodeId> for BodyMap {
    type Output = parsing::NodeId;

    fn index(&self, index: HirNodeId) -> &Self::Output {
        match index {
            HirNodeId::ItemTypeExpr(item_type_expr_id) => &self[item_type_expr_id],
            HirNodeId::TypeExpr(type_expr_id) => &self[type_expr_id],
            HirNodeId::Member(member) => &self[member],
            HirNodeId::Pat(pat_id) => &self[pat_id],
            HirNodeId::Expr(expr_id) => &self[expr_id],
            HirNodeId::Stmt(stmt_id) => &self[stmt_id],
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

impl std::ops::Index<MemberId> for BodyMap {
    type Output = parsing::NodeId;

    fn index(&self, index: MemberId) -> &Self::Output {
        &self.members[index.0]
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
