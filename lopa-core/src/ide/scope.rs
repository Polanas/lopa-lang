use std::collections::HashMap;

use la_arena::{Arena, ArenaMap, Idx};

use crate::{
    def::{
        ExprId, PatId, Symbol, SymbolList, hir,
        hir::{Enum, Expr, Function, Item, Module, Struct},
    },
    ide::{Diagnostic, DiagnosticKind, DiagnosticLocation, ResolveItem},
};

#[derive(salsa::Supertype, Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub enum ModuleDef<'db> {
    Function(Function<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    Module(Module<'db>),
}

#[derive(Clone, PartialEq, Default, salsa::SalsaValue)]
pub struct ModuleDefs<'db> {
    pub values: indexmap::IndexMap<Symbol, ModuleDef<'db>>,
    pub types: indexmap::IndexMap<Symbol, ModuleDef<'db>>,
}

impl<'db> ModuleDefs<'db> {
    pub fn resolve_item(&self, name: Symbol) -> Option<ResolveItem<'db>> {
        let value = self.values.get(&name).cloned();
        let ty = self.types.get(&name).cloned();
        Some(match (value, ty) {
            (Some(value), Some(ty)) => ResolveItem::Both { ty, value },
            (Some(value), None) => ResolveItem::Value(value),
            (None, Some(ty)) => ResolveItem::Type(ty),
            _ => return None,
        })
    }
}

#[derive(Clone, PartialEq, Default, salsa::SalsaValue)]
pub struct ModuleScope<'db> {
    pub values: indexmap::IndexMap<Symbol, ScopeName<'db>>,
    pub types: indexmap::IndexMap<Symbol, ScopeName<'db>>,

    pub global_imports: Vec<SymbolList>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'db> ModuleScope<'db> {
    pub fn value(&self, name: Symbol) -> Option<&ScopeName<'db>> {
        self.values.get(&name)
    }

    pub fn ty(&self, name: Symbol) -> Option<&ScopeName<'db>> {
        self.types.get(&name)
    }

    pub fn global_imports(&self) -> &[SymbolList] {
        &self.global_imports
    }

    pub fn types(&self) -> &indexmap::IndexMap<Symbol, ScopeName<'db>> {
        &self.types
    }
}

#[salsa::tracked]
impl<'db> Module<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn scope(self, db: &'db dyn salsa::Database) -> ModuleScope<'db> {
        let items = self.items(db).items(db);

        let mut scope = ModuleScope::default();
        let mut scope_names = ScopeNames::new(db);

        if let Some(parent) = self.parent(db)
            && matches!(self.data(db), hir::ModuleData::Definition { .. })
        {
            let parent_scope = parent.scope(db);
            for (k, v) in parent_scope.values.iter() {
                scope_names.values.insert(*k, *v);
            }
            for (k, v) in parent_scope.types.iter() {
                scope_names.types.insert(*k, *v);
            }
        }

        for item in items.iter() {
            match item {
                Item::Struct(item) => {
                    scope_names.insert_type(
                        item.name(db),
                        ScopeName::new(
                            db,
                            SymbolList::new(db, [item.name(db)]),
                            DiagnosticLocation::Struct(item.id(db)),
                        ),
                        &mut scope.diagnostics,
                    );
                }
                Item::Function(item) => {
                    scope_names.insert_value(
                        item.name(db),
                        ScopeName::new(
                            db,
                            SymbolList::new(db, [item.name(db)]),
                            DiagnosticLocation::Function(item.id(db)),
                        ),
                        &mut scope.diagnostics,
                    );
                }
                Item::Enum(item) => {
                    scope_names.insert_type(
                        item.name(db),
                        ScopeName::new(
                            db,
                            SymbolList::new(db, [item.name(db)]),
                            DiagnosticLocation::Enum(item.id(db)),
                        ),
                        &mut scope.diagnostics,
                    );
                }
                Item::Module(item) => {
                    if let Some(id) = item.id(db) {
                        scope_names.insert_type(
                            item.name(db),
                            ScopeName::new(
                                db,
                                SymbolList::new(db, [item.name(db)]),
                                DiagnosticLocation::Module(id),
                            ),
                            &mut scope.diagnostics,
                        );
                    }
                }
                Item::Use(item) => {
                    let mut traverse_ctx = TraversUseTree {
                        db,
                        use_item: *item,
                        names: &mut scope_names,
                        module: self,
                        global_imports: &mut scope.global_imports,
                        diagnostics: &mut scope.diagnostics,
                    };
                    if let Some(use_tree) = item.use_tree(db) {
                        traverse_ctx.traverse(use_tree, SymbolList::new(db, []));
                    }
                }
                Item::Impl(_) => {}
            }
        }

        scope.values = scope_names.values;
        scope.types = scope_names.types;

        scope
    }
    #[salsa::tracked(returns(ref))]
    pub fn defs(self, db: &'db dyn salsa::Database) -> ModuleDefs<'db> {
        let mut values: indexmap::IndexMap<Symbol, ModuleDef<'db>> = Default::default();
        let mut types: indexmap::IndexMap<Symbol, ModuleDef<'db>> = Default::default();
        for item in self.items(db).items(db).iter() {
            match item {
                Item::Function(item) => {
                    values.insert(item.name(db), ModuleDef::Function(*item));
                }
                Item::Struct(item) => {
                    types.insert(item.name(db), ModuleDef::Struct(*item));
                }
                Item::Enum(item) => {
                    types.insert(item.name(db), ModuleDef::Enum(*item));
                }
                Item::Module(item) => {
                    types.insert(item.name(db), ModuleDef::Module(*item));
                }
                Item::Use(_) | Item::Impl(_) => {}
            };
        }

        ModuleDefs { values, types }
    }
}

#[salsa::tracked(debug)]
pub struct ScopeName<'db> {
    #[returns(copy)]
    pub path: SymbolList,
    #[returns(clone)]
    pub location: DiagnosticLocation,
}

struct ScopeNames<'db> {
    db: &'db dyn salsa::Database,
    values: indexmap::IndexMap<Symbol, ScopeName<'db>>,
    types: indexmap::IndexMap<Symbol, ScopeName<'db>>,
}

impl<'db> ScopeNames<'db> {
    fn new(db: &'db dyn salsa::Database) -> Self {
        Self {
            db,
            values: Default::default(),
            types: Default::default(),
        }
    }

    fn insert(
        name: Symbol,
        scope_name: ScopeName<'db>,
        db: &dyn salsa::Database,
        names: &mut indexmap::IndexMap<Symbol, ScopeName<'db>>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(_) = names.insert(name, scope_name) {
            diagnostics.push(Diagnostic {
                message: format!("the name `{}` is defined multiple times", name.value(db)),
                location: scope_name.location(db),
                kind: DiagnosticKind::ModuleError,
            });
        }
    }

    fn insert_value_type(
        &mut self,
        name: Symbol,
        scope_name: ScopeName<'db>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        self.insert_value(name, scope_name, diagnostics);
        self.insert_type(name, scope_name, diagnostics);
    }

    fn insert_value(
        &mut self,
        name: Symbol,
        scope_name: ScopeName<'db>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        Self::insert(name, scope_name, self.db, &mut self.values, diagnostics);
    }

    fn insert_type(
        &mut self,
        name: Symbol,
        scope_name: ScopeName<'db>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        Self::insert(name, scope_name, self.db, &mut self.types, diagnostics);
    }
}

struct TraversUseTree<'db, 'a> {
    db: &'db dyn salsa::Database,
    use_item: hir::UseItem<'db>,
    module: hir::Module<'db>,
    names: &'a mut ScopeNames<'db>,
    global_imports: &'a mut Vec<SymbolList>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'db, 'a> TraversUseTree<'db, 'a> {
    fn traverse(&mut self, use_tree: hir::UseTree, path: SymbolList) -> Option<()> {
        match use_tree.kind(self.db) {
            hir::UseTreeKind::Name(name) => {
                let path = path.push(self.db, name);
                self.names.insert_value_type(
                    name,
                    ScopeName::new(
                        self.db,
                        path,
                        DiagnosticLocation::UseTree {
                            use_id: self.use_item.id(self.db),
                            tree_id: use_tree.id(self.db),
                        },
                    ),
                    self.diagnostics,
                );
            }
            hir::UseTreeKind::SelfUse => {
                self.names.insert_value_type(
                    *path.symbols(self.db).last().unwrap(),
                    ScopeName::new(
                        self.db,
                        path,
                        DiagnosticLocation::UseTree {
                            use_id: self.use_item.id(self.db),
                            tree_id: use_tree.id(self.db),
                        },
                    ),
                    self.diagnostics,
                );
            }
            hir::UseTreeKind::Path { name, use_tree } => {
                let path = path.push(self.db, name);
                self.traverse(use_tree, path)?;
            }
            hir::UseTreeKind::TreeList(use_tree_list) => {
                for item in use_tree_list.items(self.db).iter() {
                    self.traverse(*item, path);
                }
            }
            hir::UseTreeKind::Root { use_tree } => {
                let path = SymbolList::new(self.db, [Symbol::new(self.db, "root")]);
                self.traverse(use_tree, path);
            }
            hir::UseTreeKind::Super {
                use_tree: super_use_tree,
            } => {
                let Some(parent) = self.module.parent(self.db) else {
                    self.diagnostics.push(Diagnostic {
                        message: "too many leading `super` keywords".to_string(),
                        location: DiagnosticLocation::UseTree {
                            use_id: self.use_item.id(self.db),
                            tree_id: use_tree.id(self.db),
                        },
                        kind: DiagnosticKind::ModuleError,
                    });
                    return None;
                };
                self.module = parent;
                let path = parent.absolute_path(self.db);
                self.traverse(super_use_tree, path)?;
            }
            hir::UseTreeKind::Global => {
                self.global_imports.push(path);
            }
        }
        Some(())
    }
}

pub struct ExprScopesCtx<'db> {
    scopes: ExprScopes,
    db: &'db dyn salsa::Database,
}

impl<'db> ExprScopesCtx<'db> {
    fn new(db: &'db dyn salsa::Database) -> Self {
        Self {
            scopes: Default::default(),
            db,
        }
    }

    fn traverse_params(&mut self, params: hir::FnParamList<'db>) {
        let root = self.root_scope();
        for param in params.params(self.db) {
            match param.kind(self.db) {
                hir::FnParamKind::Pat { pat, .. } => {
                    if let Some(pat) = pat {
                        self.traverse_pat(pat, root);
                    }
                }
                hir::FnParamKind::SelfParam => {}
            }
        }
    }

    fn traverse(mut self, expr: hir::Expr<'db>) -> ExprScopes {
        let root = self.root_scope();
        self.traverse_expr(expr, root);
        self.scopes
    }

    fn traverse_expr(&mut self, expr: hir::Expr<'db>, scope: ScopeId) {
        self.scopes.scope_by_expr.insert(expr.id(self.db), scope);
        match expr.kind(self.db) {
            hir::ExprKind::Unit
            | hir::ExprKind::Lit(_)
            | hir::ExprKind::Path(_)
            | hir::ExprKind::SelfExpr => {}
            hir::ExprKind::As { expr, .. } => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::Is { expr, pat } => {
                self.traverse_expr(expr, scope);
                self.traverse_pat(pat, scope);
            }
            hir::ExprKind::IsNot { expr, pat } => {
                self.traverse_expr(expr, scope);
                self.traverse_pat(pat, scope);
            }
            hir::ExprKind::Closure { params, body, .. } => {
                let scope = self.scopes.scopes.alloc(ScopeData::from_parent(scope));
                for param in params.params(self.db).iter() {
                    self.traverse_pat(param.pattern(self.db), scope);
                }
                self.traverse_expr(body, scope);
            }
            hir::ExprKind::Field { expr, .. } => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::Method { expr, args, .. } => {
                self.traverse_expr(expr, scope);
                for arg in args.args(self.db).iter() {
                    self.traverse_expr(arg.kind(self.db).value(), scope);
                }
            }
            hir::ExprKind::Record { fields, .. } => {
                for field in fields.fields(self.db).iter() {
                    self.traverse_expr(field.expr(self.db), scope);
                }
            }
            hir::ExprKind::Binary { lhs, rhs, .. } => {
                self.traverse_expr(lhs, scope);
                self.traverse_expr(rhs, scope);
            }
            hir::ExprKind::Unary { expr, .. } => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::Index { base, index } => {
                self.traverse_expr(base, scope);
                self.traverse_expr(index, scope);
            }
            hir::ExprKind::Call { func, args } => {
                self.traverse_expr(func, scope);
                for arg in args.args(self.db).iter() {
                    self.traverse_expr(arg.kind(self.db).value(), scope);
                }
            }
            hir::ExprKind::Block { stmts } => {
                let scope = self.scopes.scopes.alloc(ScopeData::from_parent(scope));
                for stmt in stmts.stmts(self.db).iter() {
                    self.traverse_stmt(*stmt, scope);
                }
            }
            hir::ExprKind::Paren(expr) => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::Return(expr) => {
                self.traverse_expr(expr, scope);
            }
            hir::ExprKind::If(if_expr) => {
                self.traverse_if_expr(if_expr, scope);
            }
            hir::ExprKind::Loop { block } => {
                self.traverse_expr(block, scope);
            }
            hir::ExprKind::While { cond, block } => {
                self.traverse_expr(cond, scope);
                self.traverse_expr(block, scope);
            }
            hir::ExprKind::For {
                loop_expr,
                iterable,
                block,
            } => {
                self.traverse_expr(loop_expr, scope);
                self.traverse_expr(iterable, scope);
                self.traverse_expr(block, scope);
            }
            hir::ExprKind::Tuple { exprs } => {
                for expr in exprs.exprs(self.db).iter() {
                    self.traverse_expr(*expr, scope);
                }
            }
        }
    }

    fn traverse_if_expr(&mut self, expr: hir::IfExpr<'db>, scope: ScopeId) {
        self.traverse_expr(expr.cond(self.db), scope);
        self.traverse_expr(expr.if_branch(self.db), scope);
        if let Some(else_branch) = expr.else_branch(self.db) {
            match else_branch {
                hir::ElseBranch::Block(expr) => self.traverse_expr(expr, scope),
                hir::ElseBranch::If(if_expr) => self.traverse_if_expr(if_expr, scope),
            }
        }
    }

    fn traverse_stmt(&mut self, stmt: hir::Stmt<'db>, scope: ScopeId) {
        match stmt.kind(self.db) {
            hir::StmtKind::Let { pat, expr, .. } => {
                self.traverse_pat(pat, scope);
                self.traverse_expr(expr, scope);
            }
            hir::StmtKind::Expr { expr, .. } => {
                self.traverse_expr(expr, scope);
            }
        }
    }

    fn traverse_pat(&mut self, pat: hir::Pat<'db>, scope: ScopeId) {
        match pat.kind(self.db) {
            hir::PatKind::Name(symbol) => {
                self.scopes.scopes[scope].entries.push(ScopeEntry {
                    name: symbol,
                    pattern: pat.id(self.db),
                });
            }
            hir::PatKind::Wildcard => {}
            hir::PatKind::Path(_) => {}
        }
    }

    fn root_scope(&mut self) -> ScopeId {
        if !self.scopes.scopes.is_empty() {
            return self.scopes.scopes.iter().next().unwrap().0;
        }
        self.scopes.scopes.alloc(ScopeData {
            parent: None,
            entries: vec![],
        })
    }
}

#[derive(Debug, PartialEq, Eq, Default, salsa::SalsaValue)]
pub struct ExprScopes {
    scopes: Arena<ScopeData>,
    scope_by_expr: HashMap<ExprId, ScopeId>,
}

impl ExprScopes {
    pub fn entries(&self, scope: ScopeId) -> &[ScopeEntry] {
        &self.scopes[scope].entries
    }

    pub fn scope_for_expr(&self, expr_id: ExprId) -> Option<ScopeId> {
        self.scope_by_expr.get(&expr_id).copied()
    }

    pub fn scope_chain(&self, scope: Option<ScopeId>) -> impl Iterator<Item = ScopeId> {
        std::iter::successors(scope, move |&scope| self.scopes[scope].parent)
    }

    pub fn resolve_name_in_scope(&self, scope: ScopeId, name: Symbol) -> Option<&ScopeEntry> {
        self.scope_chain(Some(scope))
            .find_map(|scope| self.entries(scope).iter().rev().find(|it| it.name == name))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ScopeEntry {
    name: Symbol,
    pattern: PatId,
}

impl ScopeEntry {
    pub fn name(&self) -> Symbol {
        self.name
    }

    pub fn pattern(&self) -> PatId {
        self.pattern
    }
}

pub type ScopeId = Idx<ScopeData>;

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
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
