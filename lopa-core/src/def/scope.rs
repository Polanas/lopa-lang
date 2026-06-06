use std::sync::Arc;

use la_arena::{Arena, ArenaMap, Idx};
use ustr::Ustr;

use crate::{
    def::{
        body,
        ir::{self, ExprId, PatternId, StmtId},
        lower,
    },
    ide::{self},
    parsing::ast::{self, AstPtr},
    ustr_hash::{UstrHash, UstrIndexMap},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, salsa::Update)]
pub struct MyAstPtr<T: rowan::ast::AstNode + 'static>(pub ast::AstPtr<T>);

#[derive(salsa::Update, Clone, PartialEq, Eq, Default, Debug)]
pub struct ModuleSourceMap<'db> {
    functions: indexmap::IndexMap<MyAstPtr<ast::FnItem>, ir::Function<'db>>,
    structs: indexmap::IndexMap<MyAstPtr<ast::StructItem>, ir::Struct<'db>>,
}

impl<'db> ModuleSourceMap<'db> {
    pub fn node_to_function(&self, node: &ast::FnItem) -> Option<ir::Function<'db>> {
        let src = MyAstPtr(AstPtr::new(node));
        self.functions.get(&src).copied()
    }
}

#[derive(salsa::Update, Clone, PartialEq, Eq, Default, Debug)]
pub struct ModuleScope<'db> {
    values: UstrIndexMap<ir::ModuleValueDef<'db>>,
    types: UstrIndexMap<ir::ModuleTypeDef<'db>>,
}

impl<'db> ModuleScope<'db> {
    pub fn resolve_value(&self, name: &Ustr) -> Option<&ir::ModuleValueDef<'db>> {
        self.values.get(name)
    }

    pub fn resolve_type(&self, name: &Ustr) -> Option<&ir::ModuleTypeDef<'db>> {
        self.types.get(name)
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = (&UstrHash, &ir::ModuleValueDef)> {
        self.values.iter()
    }

    pub fn types(&self) -> impl ExactSizeIterator<Item = (&UstrHash, &ir::ModuleTypeDef)> {
        self.types.iter()
    }
}

//TODO: report collisions
#[salsa::tracked(returns(ref))]
pub fn module_scope_with_source_map<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
) -> (Arc<ModuleScope<'db>>, Arc<ModuleSourceMap<'db>>) {
    let items = lower::module_items(db, file);
    let mut source_map = ModuleSourceMap::default();
    let mut scope = ModuleScope::default();

    for func in items.functions(db) {
        source_map
            .functions
            .insert(MyAstPtr(func.ast_ptr(db).clone()), *func);
        scope
            .values
            .insert(func.name(db).into(), ir::ModuleValueDef::Function(*func));
    }

    for strct in items.structs(db) {
        source_map
            .structs
            .insert(MyAstPtr(strct.ast_ptr(db).clone()), *strct);
        scope
            .types
            .insert(strct.name(db).into(), ir::ModuleTypeDef::Struct(*strct));
    }

    for file in items.children(db) {
        let module_name = ide::module_name(db, *file);
        scope
            .types
            .insert(module_name.into(), ir::ModuleTypeDef::Module(*file));
    }

    //TODO: throw and catch errors
    for imports in items.use_imports(db) {}

    (scope.into(), source_map.into())
}

#[salsa::tracked(returns(ref))]
pub fn module_scope<'db>(db: &'db dyn salsa::Database, file: ide::File) -> Arc<ModuleScope<'db>> {
    module_scope_with_source_map(db, file).0.clone()
}

#[salsa::tracked(returns(ref))]
pub fn module_source_map<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
) -> Arc<ModuleSourceMap<'db>> {
    module_scope_with_source_map(db, file).1.clone()
}

#[salsa::tracked(returns(ref))]
pub fn expr_scopes<'db>(db: &'db dyn salsa::Database, func: ir::Function<'db>) -> Arc<ExprScopes> {
    Arc::new(ExprScopes::new(db, func))
}
struct ExprScopesCtx<'db> {
    scopes: Arena<ScopeData>,
    scope_by_expr: ArenaMap<ExprId, ScopeId>,
    body: &'db body::Body<'db>,
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
        for param in self.body.params() {
            self.add_bindings(*param, root);
        }
        self.traverse_expr(self.body.body_expr(), root);
        ExprScopes {
            scopes: self.scopes,
            scope_by_expr: self.scope_by_expr,
        }
    }

    fn traverse_expr(&mut self, expr: ExprId, scope: ScopeId) {
        self.scope_by_expr.insert(expr.into(), scope);

        match self.body.expr(expr) {
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
            ir::Expr::Call { func: expr, args } | ir::Expr::Method { expr, args, .. } => {
                for arg in args {
                    self.traverse_expr(arg.value(), scope);
                }
                self.traverse_expr(*expr, scope);
            }
            ir::Expr::If {
                if_cond,
                if_branch,
                else_branch,
            } => {
                self.traverse_expr(*if_cond, scope);
                self.traverse_expr(*if_branch, scope);
                if let Some(else_branch) = else_branch {
                    self.traverse_expr(*else_branch, scope);
                }
            }
            ir::Expr::Unary { expr, .. }
            | ir::Expr::Return { expr }
            | ir::Expr::Paren { expr }
            | ir::Expr::Field { expr, .. } => {
                self.traverse_expr(*expr, scope);
            }
            ir::Expr::Record { fields, .. } => {
                for field in fields {
                    self.traverse_expr(field.expr, scope);
                }
            }
            ir::Expr::Path(_)
            | ir::Expr::Lit(_)
            | ir::Expr::Missing
            | ir::Expr::Unit
            | ir::Expr::SelfVar => {}
        }
    }

    fn traverse_expr_stmts(&mut self, stmts: &[StmtId], scope: ScopeId) {
        for stmt_id in stmts {
            let stmt = self.body.stmt(*stmt_id);
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
        let pattern = &self.body.pattern(pattern_id);
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
            .find_map(|scope| self.entries(scope).iter().rev().find(|it| it.name == *name))
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

#[derive(Debug, PartialEq, Eq, salsa::Update)]
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
