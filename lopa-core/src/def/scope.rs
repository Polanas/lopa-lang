use std::sync::Arc;

use la_arena::{Arena, ArenaMap, Idx};
use ustr::Ustr;

use crate::{
    def::{
        body,
        ir::{self, ExprId, FileDef, PatternId},
    },
    ide::{self, lower_file},
    parsing::ast::{self, AstPtr},
    ustr_hash::{UstrHash, UstrIndexMap},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, salsa::Update)]
pub struct MyAstPtr<T: rowan::ast::AstNode + 'static>(ast::AstPtr<T>);

#[derive(salsa::Update, Clone, PartialEq, Eq, Default, Debug)]
pub struct FileSourceMap<'db> {
    functions: indexmap::IndexMap<MyAstPtr<ast::FnItem>, ir::Function<'db>>,
}

impl<'db> FileSourceMap<'db> {
    pub fn node_to_function(&self, node: &ast::FnItem) -> Option<ir::Function<'db>> {
        let src = MyAstPtr(AstPtr::new(node));
        self.functions.get(&src).copied()
    }
}

#[derive(salsa::Update, Clone, PartialEq, Eq, Default, Debug)]
pub struct FileScope<'db> {
    //uses Ustr recomputed hash
    values: UstrIndexMap<ir::FileDef<'db>>,
}

impl<'db> FileScope<'db> {
    pub fn resolve_name(&self, name: &Ustr) -> Option<&ir::FileDef<'db>> {
        self.values.get(name)
    }

    pub fn values(&self) -> impl Iterator<Item = (&UstrHash, &FileDef)> + ExactSizeIterator {
        self.values.iter()
    }
}

#[salsa::tracked(returns(ref))]
pub fn file_scope_with_source_map<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
) -> (FileScope<'db>, FileSourceMap<'db>) {
    let ir_file = lower_file(db, file);
    let mut source_map = FileSourceMap::default();
    let mut scope = FileScope::default();

    for func in ir_file.functions(db) {
        source_map
            .functions
            .insert(MyAstPtr(func.ast_ptr(db).clone()), func);
    }

    for func in ir_file.functions(db) {
        scope
            .values
            .insert(func.name(db).into(), ir::FileDef::Function(func));
    }

    (scope, source_map)
}

#[salsa::tracked(returns(ref))]
pub fn expr_scopes<'db>(db: &'db dyn salsa::Database, func: ir::Function<'db>) -> Arc<ExprScopes> {
    Arc::new(ExprScopes::new(db, func))
}

#[salsa::tracked(returns(ref))]
pub fn file_scope<'db>(db: &'db dyn salsa::Database, file: ide::File) -> FileScope<'db> {
    file_scope_with_source_map(db, file).0.clone()
}

#[salsa::tracked(returns(ref))]
pub fn file_source_map<'db>(db: &'db dyn salsa::Database, file: ide::File) -> FileSourceMap<'db> {
    file_scope_with_source_map(db, file).1.clone()
}

struct ExprScopesCtx<'db> {
    scopes: Arena<ScopeData>,
    scope_by_expr: ArenaMap<ExprId, ScopeId>,
    body: &'db body::Body,
}

impl<'db> ExprScopesCtx<'db> {
    fn new(body: &'db body::Body) -> Self {
        Self {
            scopes: Default::default(),
            scope_by_expr: Default::default(),
            body,
        }
    }

    fn traverse(mut self) -> ExprScopes {
        let root = self.root_scope();
        for param in &self.body.params {
            self.add_bindings(param.pattern, root);
        }
        self.traverse_expr(self.body.body_expr, root);
        ExprScopes {
            scopes: self.scopes,
            scope_by_expr: self.scope_by_expr,
        }
    }

    fn traverse_expr(&mut self, expr: ExprId, scope: ScopeId) {
        self.scope_by_expr.insert(expr, scope);

        match &self.body[expr] {
            ir::Expr::BlockExpr { stmts } => {
                let block_scope = self.scopes.alloc(ScopeData::from_parent(scope));
                self.traverse_expr_stmts(stmts, block_scope);
            }
            ir::Expr::Binary { left, right, .. } => {
                self.traverse_expr(*left, scope);
                self.traverse_expr(*right, scope);
            }
            ir::Expr::Index { base, index } => {
                self.traverse_expr(*base, scope);
                self.traverse_expr(*index, scope);
            }
            ir::Expr::Call { func, args } => {
                for arg in args {
                    self.traverse_expr(arg.value(), scope);
                }
                self.traverse_expr(*func, scope);
            }
            ir::Expr::If {
                if_cond,
                if_branch,
                else_branch,
            } => {
                self.traverse_expr(*if_cond, scope);
                let if_branch_scope = self.scopes.alloc(ScopeData::from_parent(scope));
                self.traverse_expr_stmts(if_branch, if_branch_scope);
                if let Some(else_branch) = else_branch {
                    match else_branch {
                        ir::ElseBranch::Else { stmts } => {
                            let else_branch_scope =
                                self.scopes.alloc(ScopeData::from_parent(scope));
                            self.traverse_expr_stmts(stmts, else_branch_scope);
                        }
                        ir::ElseBranch::ElseIf { expr } => {
                            self.traverse_expr(*expr, scope);
                        }
                    }
                }
            }
            ir::Expr::Unary { expr, .. } | ir::Expr::Return { expr } | ir::Expr::Paren { expr } => {
                self.traverse_expr(*expr, scope);
            }
            ir::Expr::Name(_) | ir::Expr::Lit(_) | ir::Expr::Missing => {}
        }
    }

    fn traverse_expr_stmts(&mut self, stmts: &[ir::Stmt], scope: ScopeId) {
        for stmt in stmts {
            match stmt {
                ir::Stmt::Let { pattern, expr, .. } => {
                    self.add_bindings(*pattern, scope);
                    self.traverse_expr(*expr, scope);
                }
                ir::Stmt::Expr { expr, .. } => {
                    self.traverse_expr(*expr, scope);
                }
            }
        }
    }

    fn add_bindings(&mut self, pattern_id: PatternId, scope: ScopeId) {
        let pattern = &self.body[pattern_id];
        match pattern {
            ir::Pattern::Missing => {}
            ir::Pattern::Name(ustr) => {
                self.scopes[scope].entries.push(ScopeEntry {
                    name: *ustr,
                    pattern: pattern_id,
                });
            }
        }
    }

    fn root_scope(&mut self) -> ScopeId {
        if !self.scopes.is_empty() {
            return self.scopes.iter().next().unwrap().0;
        }
        self.scopes.alloc(ScopeData {
            parent: None,
            entries: vec![],
        })
    }
}

#[derive(Debug, PartialEq, Eq, Default, salsa::Update)]
pub struct ExprScopes {
    scopes: Arena<ScopeData>,
    scope_by_expr: ArenaMap<ExprId, ScopeId>,
}

impl ExprScopes {
    pub fn entries(&self, scope: ScopeId) -> &[ScopeEntry] {
        &self.scopes[scope].entries
    }

    pub fn scope_for_expr(&self, expr_id: ExprId) -> Option<ScopeId> {
        self.scope_by_expr.get(expr_id).copied()
    }

    pub fn scope_chain(&self, scope: Option<ScopeId>) -> impl Iterator<Item = ScopeId> {
        std::iter::successors(scope, move |&scope| self.scopes[scope].parent)
    }

    pub fn resolve_name_in_scope(&self, scope: ScopeId, name: &Ustr) -> Option<&ScopeEntry> {
        self.scope_chain(Some(scope))
            .find_map(|scope| self.entries(scope).iter().find(|it| it.name == *name))
    }
}

impl ExprScopes {
    pub fn new<'db>(db: &'db dyn salsa::Database, func: ir::Function<'db>) -> ExprScopes {
        ExprScopesCtx::new(ide::body(db, func)).traverse()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ScopeEntry {
    name: Ustr,
    pattern: PatternId,
}

impl ScopeEntry {
    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn pattern(&self) -> Idx<ir::Pattern> {
        self.pattern
    }
}

pub type ScopeId = Idx<ScopeData>;

#[derive(Debug, PartialEq, Eq)]
pub struct ScopeData {
    parent: Option<ScopeId>,
    entries: Vec<ScopeEntry>,
}

impl ScopeData {
    fn from_parent(parent: ScopeId) -> Self {
        Self {
            parent: Some(parent),
            entries: Default::default(),
        }
    }
}
