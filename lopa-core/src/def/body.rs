use la_arena::Arena;

use crate::{
    def::ir::{self, ExprId},
    ide::base::InFile,
    parsing::ast::{self, AstPtr},
};

pub type ExprPtr = AstPtr<ast::Expr>;
pub type ExprSource = InFile<ExprPtr>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub pattern: ir::Pattern,
    pub type_expr: ir::TypeExpr,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Body {
    pub exprs: Arena<ir::Expr>,
    pub patterns: Arena<ir::Pattern>,
    pub params: Vec<Param>,
    pub output: Option<ir::TypeExpr>,
    pub body_expr: ExprId,
}

// impl Default for Bo

// pub struct BodySourceMap {
//     map: Arena
// }
