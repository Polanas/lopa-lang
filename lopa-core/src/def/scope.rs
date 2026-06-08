use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use la_arena::{Arena, ArenaMap, Idx};
use notify_rust::Notification;
use rowan::ast::AstNode;
use salsa::Accumulator;
use ustr::Ustr;

use crate::{
    def::{
        body,
        ir::{self, ExprId, PatternId, StmtId},
        lower, resolver,
    },
    ide::{
        self,
        diagnostics::{Diagnostic, DiagnosticKind},
    },
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
    scope_names: UstrIndexMap<ir::Path>,
}

impl<'db> ModuleScope<'db> {
    pub fn resolve_value(&self, name: &Ustr) -> Option<&ir::ModuleValueDef<'db>> {
        self.values.get(name)
    }

    pub fn resolve_type(&self, name: &Ustr) -> Option<&ir::ModuleTypeDef<'db>> {
        self.types.get(name)
    }

    pub fn resolve_name(&self, name: &Ustr) -> Option<&ir::Path> {
        self.scope_names.get(name)
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
    let parse = ide::parse(db, file);

    for func in items.functions(db) {
        source_map
            .functions
            .insert(MyAstPtr(func.ast_ptr(db).clone()), *func);
        scope
            .values
            .insert(func.name(db).into(), ir::ModuleValueDef::Function(*func));
        scope
            .scope_names
            .insert(func.name(db).into(), ir::Path(vec![func.name(db)]));
    }

    for strct in items.structs(db) {
        source_map
            .structs
            .insert(MyAstPtr(strct.ast_ptr(db).clone()), *strct);
        scope
            .types
            .insert(strct.name(db).into(), ir::ModuleTypeDef::Struct(*strct));
        scope
            .scope_names
            .insert(strct.name(db).into(), ir::Path(vec![strct.name(db)]));
    }

    for file in items.children(db) {
        let module_name = ide::module_name(db, *file);
        scope
            .types
            .insert(module_name.into(), ir::ModuleTypeDef::Module(*file));
        scope
            .scope_names
            .insert(module_name.into(), ir::Path(vec![module_name]));
    }

    for import in items.use_imports(db) {
        let Some(use_tree) = import
            .ast_ptr(db)
            .to_node(&parse.syntax_node(db))
            .use_tree()
        else {
            continue;
        };
        traverse_use_tree(
            db,
            file,
            &use_tree,
            &ir::Path(vec![]),
            &mut scope.scope_names,
        );
    }

    (scope.into(), source_map.into())
}

#[salsa::tracked]
pub fn resolve_imports(db: &dyn salsa::Database, file: ide::File) {
    let items = lower::module_items(db, file);
    let parse = ide::parse(db, file);
    for import in items.use_imports(db) {
        let Some(use_tree) = import
            .ast_ptr(db)
            .to_node(&parse.syntax_node(db))
            .use_tree()
        else {
            continue;
        };
        resolve_use_tree(db, file, &use_tree, ir::Path(vec![]));
    }
}

fn resolve_use_tree(
    db: &dyn salsa::Database,
    file: ide::File,
    tree: &ast::UseTree,
    path: ir::Path,
) -> Option<()> {
    match tree {
        ast::UseTree::UseName(use_name) => {
            let mut path = path.clone();
            let name = use_name.name()?.text()?;
            path.0.push(name);

            if resolver::resolve_path(db, file, path).is_none() {
                Diagnostic::new(
                    use_name.syntax().text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                )
                .accumulate(db);
            }
        }
        ast::UseTree::UseSelfName(use_self_name) => {
            let mut path = path.clone();
            let name = Ustr::from("self");
            path.0.push(name);

            if resolver::resolve_path(db, file, path).is_none() {
                Diagnostic::new(
                    use_self_name.syntax().text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                )
                .accumulate(db);
            }
        }
        ast::UseTree::UsePath(use_path) => {
            let mut path = path.clone();
            let name = use_path.name()?.text()?;
            path.0.push(name);
            if resolver::resolve_path(db, file, path.clone()).is_none() {
                Diagnostic::new(
                    use_path.name()?.syntax().text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                )
                .accumulate(db);
            }
            resolve_use_tree(db, file, &use_path.use_tree()?, path)?;
        }
        ast::UseTree::UseRootPath(use_root_path) => {
            let mut path = path.clone();
            let name = "root".into();
            path.0.push(name);
            if resolver::resolve_path(db, file, path.clone()).is_none() {
                Diagnostic::new(
                    use_root_path.root_token()?.text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                )
                .accumulate(db);
            }
            resolve_use_tree(db, file, &use_root_path.use_tree()?, path)?;
        }
        ast::UseTree::UseTreeList(use_tree_list) => {
            for elem in use_tree_list.elements() {
                resolve_use_tree(db, file, &elem, path.clone());
            }
            return None;
        }
        ast::UseTree::UseSuperPath(use_super_path) => {
            let mut path = path.clone();
            let name = "root".into();
            path.0.push(name);
            if resolver::resolve_path(db, file, path.clone()).is_none() {
                Diagnostic::new(
                    use_super_path.super_token()?.text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                )
                .accumulate(db);
            }
            resolve_use_tree(db, file, &use_super_path.use_tree()?, path)?;
        }
        ast::UseTree::UseGlobal(use_global) => todo!(),
    };
    Some(())
}

fn traverse_use_tree(
    db: &dyn salsa::Database,
    file: ide::File,
    tree: &ast::UseTree,
    path: &ir::Path,
    names: &mut UstrIndexMap<ir::Path>,
) -> Option<()> {
    match tree {
        ast::UseTree::UseName(use_name) => {
            let mut path = path.clone();
            let name = use_name.name()?.text()?;
            path.0.push(name);
            names.insert(name.into(), path);
        }
        ast::UseTree::UseSelfName(_) => {
            let path = path.clone();
            let name = *path.0.last().unwrap();
            names.insert(name.into(), path);
        }
        ast::UseTree::UsePath(use_path) => {
            let mut path = path.clone();
            path.0.push(use_path.name()?.text()?);
            traverse_use_tree(db, file, &use_path.use_tree()?, &path, names)?;
        }
        ast::UseTree::UseTreeList(use_tree_list) => {
            for elem in use_tree_list.elements() {
                traverse_use_tree(db, file, &elem, &path, names);
            }
            return None;
        }
        ast::UseTree::UseRootPath(use_root_path) => {
            let mut path = path.clone();
            path.0.push(Ustr::from("root"));
            traverse_use_tree(db, file, &use_root_path.use_tree()?, &path, names)?;
        }
        ast::UseTree::UseSuperPath(use_super_path) => {
            let mut path = ir::Path(vec![]);
            let mut current = file;
            while let Some(parent) = lower::module_parent(db, current) {
                path.0.insert(0, ide::module_name(db, parent));
                current = parent;
            }
            traverse_use_tree(db, file, &use_super_path.use_tree()?, &path, names)?;
        }
        ast::UseTree::UseGlobal(use_global) => todo!(),
    };
    Some(())
}

// #[derive(Clone, Debug)]
// struct UseItem {
//     kind: UseItemKind,
//     children: HashMap<Ustr, Self>,
// }
//
// impl UseItem {
//     fn new(kind: UseItemKind) -> Self {
//         Self {
//             kind,
//             children: Default::default(),
//         }
//     }
//
//     fn add_child(&mut self, item: Self) {
//         self.children.insert(
//             match &item.kind {
//                 UseItemKind::Name(ustr) => *ustr,
//                 UseItemKind::Root => "root".into(),
//                 UseItemKind::Itself => "self".into(),
//                 UseItemKind::Global => "*".into(),
//             },
//             item,
//         );
//     }
// }

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
        self.scope_by_expr.insert(expr, scope);

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
            ir::Expr::As { expr, .. } => {
                self.traverse_expr(*expr, scope);
            }
            ir::Expr::Closure { params, output } => {}
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
