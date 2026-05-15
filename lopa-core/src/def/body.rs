use std::{collections::HashMap, f32::consts::E};

use itertools::Itertools;
use la_arena::{Arena, ArenaMap, RawIdx};
use rowan::ast::AstPtr;
use salsa::Database;
use ustr::Ustr;

use crate::{
    def::{
        self,
        ir::{self, Arg, Expr, ExprId, Pattern, PatternId, Stmt},
        lower,
    },
    ide::{self, base::InFile, lower_file},
    parsing::ast::{self},
};

pub type ExprPtr = AstPtr<ast::Expr>;
pub type ExprSource = InFile<ExprPtr>;

pub type PatternPtr = AstPtr<ast::Pattern>;
pub type PatternSource = InFile<PatternPtr>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub pattern: Pattern,
    pub type_expr: ir::TypeExpr,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Body {
    pub exprs: Arena<Expr>,
    pub patterns: Arena<Pattern>,
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

impl BodyLowerCtx {
    fn alloc_expr(&mut self, expr: Expr, ptr: AstPtr<ast::Expr>) -> ExprId {
        let ptr = InFile::new(self.file, ptr);
        let id = self.body.exprs.alloc(expr);
        self.source_map.expr_source_to_id.insert(ptr.clone(), id);
        self.source_map.expr_id_to_source.insert(id, ptr.clone());
        id
    }

    fn alloc_pattern(&mut self, pattern: Pattern, ptr: AstPtr<ast::Pattern>) -> PatternId {
        let ptr = InFile::new(self.file, ptr);
        let id = self.body.patterns.alloc(pattern);
        self.source_map.pattern_source_to_id.insert(ptr.clone(), id);
        self.source_map.pattern_id_to_source.insert(id, ptr.clone());
        id
    }

    fn lower_expr(&mut self, expr: ast::Expr) -> ExprId {
        let ptr = AstPtr::new(&expr);
        match expr {
            ast::Expr::NameExpr(name_expr) => {
                let expr = name_expr
                    .name()
                    .and_then(|n| n.text())
                    .map(|n| Expr::Name(n))
                    .unwrap_or_else(|| Expr::Missing);
                self.alloc_expr(expr, ptr)
            }
            ast::Expr::BinaryExpr(binary_expr) => {
                let Some(kind) = binary_expr.op_kind() else {
                    return self.alloc_expr(Expr::Missing, ptr);
                };
                let left = self.lower_expr_opt(binary_expr.lhs());
                let right = self.lower_expr_opt(binary_expr.rhs());

                self.alloc_expr(Expr::Binary { left, right, kind }, ptr)
            }
            ast::Expr::UnaryExpr(unary_expr) => {
                let Some(kind) = unary_expr.op_kind() else {
                    return self.alloc_expr(Expr::Missing, ptr);
                };
                let expr = self.lower_expr_opt(unary_expr.expr());
                self.alloc_expr(Expr::Unary { expr, kind }, ptr)
            }
            ast::Expr::BlockExpr(block_expr) => {
                let stmts = block_expr
                    .stmts()
                    .map(|s| self.lower_stmt(s))
                    .flatten()
                    .collect_vec();
                self.alloc_expr(Expr::BlockExpr { stmts }, ptr)
            }
            ast::Expr::IndexExpr(index_expr) => {
                let base = self.lower_expr_opt(index_expr.base());
                let index = self.lower_expr_opt(index_expr.index());
                self.alloc_expr(Expr::Index { base, index }, ptr)
            }
            ast::Expr::CallExpr(call_expr) => {
                let func = self.lower_expr_opt(call_expr.func());
                let args = call_expr
                    .args()
                    .map(|l| {
                        l.args()
                            .map(|arg| {
                                if let Some(label) = arg.label().and_then(|l| l.text()) {
                                    let value = arg.value().map(|e| self.lower_expr(e));
                                    Arg::Labeled { label, value }
                                } else {
                                    Arg::NonLabeled {
                                        value: self.lower_expr_opt(arg.value()),
                                    }
                                }
                            })
                            .collect_vec()
                    })
                    .unwrap_or_else(|| vec![]);
                self.alloc_expr(Expr::Call { func, args }, ptr)
            }
            ast::Expr::ParenExpr(paren_expr) => {
                let expr = self.lower_expr_opt(paren_expr.expr());
                self.alloc_expr(Expr::Paren { expr }, ptr)
            }
            ast::Expr::ReturnExpr(return_expr) => {
                let expr = self.lower_expr_opt(return_expr.expr());
                self.alloc_expr(Expr::Return { expr }, ptr)
            }
            ast::Expr::LitExpr(lit_expr) => self.alloc_expr(
                lit_expr
                    .kind()
                    .map(|k| Expr::Lit(k))
                    .unwrap_or_else(|| Expr::Missing),
                ptr,
            ),
            ast::Expr::TryExpr(try_expr) => {
                // let expr = self.lower_expr_opt(try_expr.expr());
                //TODO: implement try expr
                self.alloc_expr(Expr::Missing, ptr)
            }
        }
    }

    fn lower_expr_opt(&mut self, expr: Option<ast::Expr>) -> ExprId {
        let Some(expr) = expr else {
            return self.body.exprs.alloc(Expr::Missing);
        };
        self.lower_expr(expr)
    }

    fn lower_pattern_opt(&mut self, pattern: Option<ast::Pattern>) -> PatternId {
        let Some(pattern) = pattern else {
            return self.body.patterns.alloc(Pattern::Missing);
        };
        self.lower_pattern(pattern)
    }

    fn lower_pattern(&mut self, pattern: ast::Pattern) -> PatternId {
        let ptr = AstPtr::new(&pattern);
        match pattern {
            ast::Pattern::NamePattern(name_pattern) => {
                let pattern = name_pattern
                    .name()
                    .and_then(|n| n.text())
                    .map(|n| Pattern::Name(n))
                    .unwrap_or_else(|| Pattern::Missing);
                self.alloc_pattern(pattern, ptr)
            }
        }
    }

    fn lower_stmt(&mut self, stmt: ast::Stmt) -> Option<ir::Stmt> {
        Some(match stmt {
            ast::Stmt::LetStmt(let_stmt) => {
                let (Some(pattern), Some(expr)) = (let_stmt.pattern(), let_stmt.expr()) else {
                    return None;
                };
                let pattern = self.lower_pattern(pattern);
                let expr = self.lower_expr(expr);
                Stmt::Let {
                    pattern,
                    expr,
                    ty: let_stmt.ty().and_then(|t| lower::lower_type_expr(t)),
                }
            }
            ast::Stmt::ExprStmt(expr_stmt) => {
                let Some(expr) = expr_stmt.expr() else {
                    return None;
                };
                Stmt::Expr {
                    expr: self.lower_expr(expr),
                    semi: expr_stmt.semi_token().map(|_| ()),
                }
            }
        })
    }
}

#[derive(Default)]
pub struct BodySourceMap {
    expr_source_to_id: HashMap<ExprSource, ExprId>,
    expr_id_to_source: ArenaMap<ExprId, ExprSource>,
    pattern_source_to_id: HashMap<PatternSource, PatternId>,
    pattern_id_to_source: ArenaMap<PatternId, PatternSource>,
}

impl BodySourceMap {
    pub fn expr_for_node(&self, node: InFile<&ast::Expr>) -> Option<ExprId> {
        let src = node.map(AstPtr::new);
        self.expr_source_to_id.get(&src).cloned()
    }

    pub fn node_for_expr(&self, expr_id: ExprId) -> Option<ExprSource> {
        self.expr_id_to_source.get(expr_id).cloned()
    }

    pub fn pattern_for_node(&self, node: InFile<&ast::Pattern>) -> Option<PatternId> {
        let src = node.map(AstPtr::new);
        self.pattern_source_to_id.get(&src).cloned()
    }

    pub fn node_for_pattern(&self, pat_id: PatternId) -> Option<PatternSource> {
        self.pattern_id_to_source.get(pat_id).cloned()
    }
}

pub fn lower<'a>(db: &'a dyn Database, function: &'a ir::Function) -> (Body, BodySourceMap) {
    let file_id = function.file(db);
    let file_ir = ide::lower_file(db, file_id);
    let parse = ide::parse(db, file_id);
    let ast = function.ast_ptr(db).to_node(&parse.syntax_node(db));

    let ctx = BodyLowerCtx {
        body: Default::default(),
        source_map: BodySourceMap::default(),
        file: file_id,
    };
    todo!()
}
