use std::{collections::HashMap, mem::transmute, ops::Index};

use itertools::Itertools;
use la_arena::{Arena, ArenaMap, Idx, RawIdx};
use rowan::ast::{AstNode as _, AstPtr};
use salsa::Database;
use ustr::Ustr;

use crate::{
    def::{
        ir::{self, Arg, Expr, ExprId, Pattern, PatternId, Stmt, StmtId, Type},
        lower::{self, lower_type_expr},
        scope::MyAstPtr,
    },
    ide::{self, base::InFile},
    parsing::ast::{self},
};

pub type ExprPtr = AstPtr<ast::Expr>;
pub type ExprSource = InFile<ExprPtr>;

pub type StmtPtr = MyAstPtr<ast::Stmt>;
pub type StmtSource = InFile<StmtPtr>;

pub type PatternPtr = AstPtr<ast::Pattern>;
pub type PatternSource = InFile<PatternPtr>;

#[derive(Clone, Debug, PartialEq, Eq, salsa::Update)]
pub struct Param<'db> {
    pub pattern: PatternId,
    pub type_expr: Type<'db>,
}

#[derive(PartialEq, Eq, Debug, Clone, salsa::Update)]
pub struct Body<'db> {
    exprs: Arena<Expr>,
    patterns: Arena<Pattern>,
    stmts: Arena<Stmt<'db>>,
    params: Vec<PatternId>,
    body_expr: ExprId,
}

impl<'db> Body<'db> {
    pub fn pattern(&self, index: PatternId) -> &Pattern {
        &self.patterns[index]
    }

    pub fn expr(&self, index: ExprId) -> &Expr {
        &self.exprs[index]
    }

    pub fn stmt(&self, index: StmtId) -> &Stmt<'_> {
        &self.stmts[Idx::from_raw(index)]
    }

    pub fn body_expr(&self) -> ExprId {
        self.body_expr
    }

    pub fn params(&'_ self) -> &'_ [PatternId] {
        &self.params
    }
}

impl<'db> Default for Body<'db> {
    fn default() -> Self {
        Self {
            exprs: Default::default(),
            patterns: Default::default(),
            params: Default::default(),
            stmts: Default::default(),
            //HACK: implementing Defualt without optional ExprId
            body_expr: ExprId::from_raw(RawIdx::from(0)),
        }
    }
}

struct BodyLowerCtx<'db> {
    db: &'db dyn salsa::Database,
    body: Body<'db>,
    source_map: BodySourceMap<'db>,
    file: ide::File,
}

impl<'db> BodyLowerCtx<'db> {
    fn alloc_expr(&mut self, expr: Expr, ptr: AstPtr<ast::Expr>) -> ExprId {
        let ptr = InFile::new(self.file, ptr);
        let id = self.body.exprs.alloc(expr);
        self.source_map.expr_source_to_id.insert(ptr.clone(), id);
        self.source_map.expr_id_to_source.insert(id, ptr.clone());
        id
    }

    fn alloc_stmt(&mut self, stmt: Stmt<'db>, ptr: AstPtr<ast::Stmt>) -> StmtId {
        let ptr = InFile::new(self.file, MyAstPtr(ptr));
        let id = self.body.stmts.alloc(stmt);
        self.source_map
            .stmt_source_to_id
            .insert(ptr.clone(), id.into_raw());
        self.source_map.stmt_id_to_source.insert(id, ptr.clone());
        id.into_raw()
    }

    fn missing_expr(&mut self, ptr: AstPtr<ast::Expr>) -> ExprId {
        self.alloc_expr(Expr::Missing, ptr)
    }

    fn alloc_pattern(&mut self, pattern: Pattern, ptr: AstPtr<ast::Pattern>) -> PatternId {
        let ptr = InFile::new(self.file, ptr);
        let id = self.body.patterns.alloc(pattern);
        self.source_map.pat_source_to_id.insert(ptr.clone(), id);
        self.source_map.pat_id_to_source.insert(id, ptr.clone());
        id
    }

    fn lower_expr(&mut self, expr: ast::Expr) -> ExprId {
        let ptr = AstPtr::new(&expr);
        match expr {
            ast::Expr::PathExpr(path_expr) => {
                let expr = path_expr
                    .path()
                    .map(|n| n.segments().collect_vec())
                    .map(ir::Path)
                    .map(Expr::Path)
                    .unwrap_or_else(|| Expr::Missing);
                self.alloc_expr(expr, ptr)
            }
            ast::Expr::BinaryExpr(binary_expr) => {
                let Some(kind) = binary_expr.op_kind() else {
                    return self.missing_expr(ptr);
                };
                let left = self.lower_expr_opt(binary_expr.lhs());
                let right = self.lower_expr_opt(binary_expr.rhs());

                self.alloc_expr(Expr::Binary { left, right, kind }, ptr)
            }
            ast::Expr::UnaryExpr(unary_expr) => {
                let Some(kind) = unary_expr.op_kind() else {
                    return self.missing_expr(ptr);
                };
                let expr = self.lower_expr_opt(unary_expr.expr());
                self.alloc_expr(Expr::Unary { expr, kind }, ptr)
            }
            ast::Expr::BlockExpr(block_expr) => {
                let block = self.lower_block(&block_expr);
                self.alloc_expr(block, ptr)
            }
            ast::Expr::IndexExpr(index_expr) => {
                let base = self.lower_expr_opt(index_expr.base());
                let index = self.lower_expr_opt(index_expr.index());
                self.alloc_expr(Expr::Index { base, index }, ptr)
            }
            ast::Expr::CallExpr(call_expr) => {
                let func = self.lower_expr_opt(call_expr.func());
                let args = self.lower_args(call_expr.args());
                self.alloc_expr(Expr::Call { func, args }, ptr)
            }
            ast::Expr::IfExpr(if_expr) => {
                let if_cond = self.lower_expr_opt(if_expr.if_condition());
                let Some(if_branch) = if_expr.if_branch().map(|b| {
                    let block = self.lower_block(&b);
                    let ptr = AstPtr::new(&ast::Expr::BlockExpr(b));
                    self.alloc_expr(block, ptr)
                }) else {
                    return self.alloc_expr(Expr::Missing, ptr);
                };
                let else_branch = if_expr.else_token().and_then(|_| {
                    if let Some(else_if_expr) = if_expr.else_if_expr() {
                        let expr = self.lower_expr(ast::Expr::IfExpr(else_if_expr));
                        Some(expr)
                    } else {
                        if let Some(else_branch) = if_expr.else_branch() {
                            let block = self.lower_block(&else_branch);
                            let ptr = AstPtr::new(&ast::Expr::BlockExpr(else_branch));
                            Some(self.alloc_expr(block, ptr))
                        } else {
                            None
                        }
                    }
                });
                self.alloc_expr(
                    Expr::If {
                        if_cond,
                        if_branch,
                        else_branch,
                    },
                    ptr,
                )
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
                    .map(Expr::Lit)
                    .unwrap_or_else(|| Expr::Missing),
                ptr,
            ),
            ast::Expr::UnitExpr(_) => self.alloc_expr(Expr::Unit, ptr),
            ast::Expr::TryExpr(try_expr) => {
                // let expr = self.lower_expr_opt(try_expr.expr());
                //TODO: implement try expr
                self.missing_expr(ptr)
            }
            ast::Expr::FieldExpr(field_expr) => {
                let Some(name) = field_expr.name().and_then(|n| n.text()) else {
                    return self.missing_expr(ptr);
                };
                let expr = self.lower_expr_opt(field_expr.expr());
                self.alloc_expr(Expr::Field { name, expr }, ptr)
            }
            ast::Expr::MethodExpr(method_expr) => {
                let Some(name) = method_expr.name().and_then(|n| n.text()) else {
                    return self.missing_expr(ptr);
                };
                let expr = self.lower_expr_opt(method_expr.expr());
                let args = self.lower_args(method_expr.args());
                self.alloc_expr(Expr::Method { name, expr, args }, ptr)
            }
            ast::Expr::RecordExpr(record_expr) => {
                let Some(path) = record_expr.path().map(|p| p.segments().collect_vec()) else {
                    return self.missing_expr(ptr);
                };
                let fields = record_expr
                    .fields_list()
                    .filter_map(|field| self.lower_field(field))
                    .collect_vec();
                self.alloc_expr(Expr::Record { path, fields }, ptr)
            }
            ast::Expr::SelfExpr(_) => self.alloc_expr(Expr::SelfVar, ptr),
            ast::Expr::AsExpr(as_expr) => {
                let Some(ty) = as_expr.type_expr() else {
                    return self.missing_expr(ptr);
                };
                let expr = self.lower_expr_opt(as_expr.expr());
                let ty = lower::lower_type_expr(self.db, self.file, ty);
                self.alloc_expr(
                    Expr::As {
                        expr,
                        ty: unsafe { transmute::<Type<'_>, Type<'static>>(ty) },
                    },
                    ptr,
                )
            }
            ast::Expr::IsExpr(is_expr) => {
                let pat = self.lower_pattern_opt(is_expr.pat());
                let expr = self.lower_expr_opt(is_expr.expr());
                self.alloc_expr(Expr::Is { expr, pat }, ptr)
            }
            ast::Expr::IsNotExpr(is_not_expr) => {
                let pat = self.lower_pattern_opt(is_not_expr.pat());
                let expr = self.lower_expr_opt(is_not_expr.expr());
                self.alloc_expr(Expr::IsNot { expr, pat }, ptr)
            }
            ast::Expr::ClosureExpr(closure_expr) => {
                //TODO: closure
                self.missing_expr(ptr)
            }
        }
    }

    fn lower_block(&mut self, block: &ast::BlockExpr) -> Expr {
        Expr::BlockExpr {
            stmts: block
                .stmts()
                .filter_map(|s| self.lower_stmt(s))
                .collect_vec(),
        }
    }

    fn lower_field(&mut self, field: ast::RecordField) -> Option<ir::RecordField> {
        let name = field.name().and_then(|n| n.text())?;
        let expr = self.lower_expr_opt(field.expr());
        Some(ir::RecordField { name, expr })
    }

    fn lower_args(&mut self, args: impl Iterator<Item = ast::Arg>) -> Vec<Arg> {
        args.map(|arg| {
            if let Some(label) = arg.label().and_then(|l| l.text()) {
                let value = self.lower_expr_opt(arg.value());
                Arg::Labeled { label, value }
            } else {
                Arg::NonLabeled {
                    value: self.lower_expr_opt(arg.value()),
                }
            }
        })
        .collect_vec()
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
                    .map(Pattern::Name)
                    .unwrap_or_else(|| Pattern::Missing);
                self.alloc_pattern(pattern, ptr)
            }
            ast::Pattern::PathPattern(path_pattern) => {
                let pattern = path_pattern
                    .path()
                    .map(|p| p.segments().collect_vec())
                    .map(ir::Path)
                    .map(Pattern::Path)
                    .unwrap_or_else(|| Pattern::Missing);
                self.alloc_pattern(pattern, ptr)
            }
            ast::Pattern::WildcardPattern(_) => self.alloc_pattern(Pattern::Wildcard, ptr),
        }
    }

    fn lower_stmt(&mut self, stmt: ast::Stmt) -> Option<StmtId> {
        let ptr = AstPtr::new(&stmt);
        Some(match stmt {
            ast::Stmt::LetStmt(let_stmt) => {
                let (Some(pattern), Some(expr)) = (let_stmt.pattern(), let_stmt.expr()) else {
                    return None;
                };
                let pattern = self.lower_pattern(pattern);
                let expr = self.lower_expr(expr);
                self.alloc_stmt(
                    Stmt::Let {
                        pat: pattern,
                        expr,
                        ty: let_stmt
                            .ty()
                            .map(|ty| lower::lower_type_expr(self.db, self.file, ty)),
                    },
                    ptr,
                )
            }
            ast::Stmt::ExprStmt(expr_stmt) => {
                let expr = self.lower_expr(expr_stmt.expr()?);
                self.alloc_stmt(
                    Stmt::Expr {
                        expr,
                        semi: expr_stmt.semi_token().map(|_| ()),
                    },
                    ptr,
                )
            }
        })
    }
}

#[derive(Default, PartialEq, Eq, Clone, salsa::Update)]
pub struct BodySourceMap<'db> {
    expr_source_to_id: HashMap<ExprSource, ExprId>,
    expr_id_to_source: ArenaMap<ExprId, ExprSource>,
    pat_source_to_id: HashMap<PatternSource, PatternId>,
    pat_id_to_source: ArenaMap<PatternId, PatternSource>,
    stmt_source_to_id: HashMap<StmtSource, StmtId>,
    stmt_id_to_source: ArenaMap<Idx<Stmt<'db>>, StmtSource>,
}

impl<'db> BodySourceMap<'db> {
    pub fn expr_for_node(&self, node: InFile<&ast::Expr>) -> Option<ExprId> {
        let src = node.map(AstPtr::new);
        self.expr_source_to_id.get(&src).cloned()
    }

    pub fn node_for_expr(&self, expr_id: ExprId) -> Option<ExprSource> {
        self.expr_id_to_source.get(expr_id).cloned()
    }

    pub fn stmt_for_node(&self, node: InFile<&ast::Stmt>) -> Option<StmtId> {
        let src = node.map(AstPtr::new).map(MyAstPtr);
        self.stmt_source_to_id.get(&src).cloned()
    }

    pub fn node_for_stmt(&self, stmt_id: StmtId) -> Option<StmtSource> {
        self.stmt_id_to_source.get(Idx::from_raw(stmt_id)).cloned()
    }

    pub fn pattern_for_node(&self, node: InFile<&ast::Pattern>) -> Option<PatternId> {
        let src = node.map(AstPtr::new);
        self.pat_source_to_id.get(&src).cloned()
    }

    pub fn node_for_pattern(&self, pat_id: PatternId) -> Option<PatternSource> {
        self.pat_id_to_source.get(pat_id).cloned()
    }
}

pub fn stmt_node<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    stmt: StmtId,
) -> Option<ast::SyntaxNode> {
    let source_map = ide::source_map(db, func);
    let parse = ide::parse(db, func.file(db));
    source_map
        .node_for_stmt(stmt)
        .map(|n| n.value.0.syntax_node_ptr().to_node(&parse.syntax_node(db)))
}

pub fn expr_node<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<ast::SyntaxNode> {
    let source_map = ide::source_map(db, func);
    let parse = ide::parse(db, func.file(db));
    source_map
        .node_for_expr(expr)
        .map(|n| n.value.syntax_node_ptr().to_node(&parse.syntax_node(db)))
}

pub fn expr_range<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<rowan::TextRange> {
    expr_node(db, func, expr).map(|node| node.text_range())
}

pub fn stmt_type_range<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    stmt: StmtId,
) -> Option<rowan::TextRange> {
    let stmt = ast::Stmt::cast(stmt_node(db, func, stmt)?)?;
    let ast::Stmt::LetStmt(let_stmt) = stmt else {
        return None;
    };
    let_stmt.ty().map(|ty| ty.syntax().text_range())
}

pub fn binary_op_range<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<rowan::TextRange> {
    let node = expr_node(db, func, expr)?;
    ast::BinaryExpr(node.parent()?)
        .op_token()
        .map(|t| t.text_range())
}

pub fn expr_text<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    expr: ExprId,
) -> Option<Ustr> {
    let node = expr_node(db, func, expr)?;
    Some(node.text().to_string().into())
}

pub fn stmt_type_text<'db>(
    db: &'db dyn salsa::Database,
    func: ir::Function<'db>,
    stmt: StmtId,
) -> Option<Ustr> {
    let node = expr_node(db, func, stmt.into())?;
    Some(node.text().to_string().into())
}

pub fn lower<'db>(
    db: &'db dyn Database,
    function: ir::Function<'db>,
) -> (Body<'db>, BodySourceMap<'db>) {
    let file = function.file(db);
    let parse = ide::parse(db, file);
    let ast = function.ast_ptr(db).to_node(&parse.syntax_node(db));

    let mut ctx = BodyLowerCtx {
        body: Default::default(),
        source_map: BodySourceMap::default(),
        file,
        db,
    };
    if let Some(params) = ast.params() {
        for ast_param in params.params() {
            if let Some(pattern) = ast_param.pattern() {
                let pattern = ctx.lower_pattern(pattern);
                ctx.body.params.push(pattern);
            }
        }
    }

    let expr_id = ctx.lower_expr_opt(ast.body().map(ast::Expr::BlockExpr));
    ctx.body.body_expr = expr_id;

    (ctx.body, ctx.source_map)
}
