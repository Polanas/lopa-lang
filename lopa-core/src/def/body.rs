use std::collections::HashMap;

use la_arena::{Arena, ArenaMap, RawIdx};

use crate::{
    def::ir::{self, ExprId},
    ide::{self, base::InFile},
    parsing::ast::{self, AstPtr},
};

pub type ExprPtr = AstPtr<ast::Expr>;
pub type ExprSource = InFile<ExprPtr>;

// pub type PatternPtr = AstPtr<ast::Pattern>;
// pub type PatternSource = InFile<PatternPtr>;

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

impl Default for Body {
    fn default() -> Self {
        Self {
            exprs: Default::default(),
            patterns: Default::default(),
            params: Default::default(),
            output: Default::default(),
            //HACK: implementing Defualt without optional ExprId
            body_expr: ExprId::from_raw(RawIdx::from(u32::MAX)),
        }
    }
}

struct BodyLowerCtx {
    body: Body,
    source_map: BodySourceMap,
    file: ide::File,
}

pub struct BodySourceMap {
    expr_map: HashMap<ExprSource, ExprId>,
    expr_map_rev: ArenaMap<ExprId, ExprSource>,
    // pattern_map: HashMap<>
}
