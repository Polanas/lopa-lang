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
        ir::{self, ExprId, ModuleDefKind, PatternId, StmtId},
        lower, resolver,
    },
    ide::{
        self,
        diagnostics::{Diagnostic, DiagnosticKind},
    },
    parsing::ast::{self, AstPtr},
    range_max::RangeMax,
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
    //items declared inside the module
    values: UstrIndexMap<ir::ModuleDef<'db>>,
    types: UstrIndexMap<ir::ModuleDef<'db>>,

    //all visible items, including imports
    scope_values: UstrIndexMap<ScopeName>,
    scope_types: UstrIndexMap<ScopeName>,

    global_imports: Vec<ir::Path>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(salsa::Update, Clone, PartialEq, Eq, Default, Debug)]
pub struct ScopeName {
    pub path: ir::Path,
    pub range: rowan::TextRange,
    pub item: Option<ModuleDefKind>,
}

struct ScopeNames<'db> {
    db: &'db dyn salsa::Database,
    values: UstrIndexMap<ScopeName>,
    types: UstrIndexMap<ScopeName>,
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
        name: Ustr,
        scope_name: ScopeName,
        db: &dyn salsa::Database,
        names: &mut UstrIndexMap<ScopeName>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(old) = names.insert(name.into(), scope_name.clone()) {
            let range = match (old.item, scope_name.item) {
                (Some(old), Some(new)) if old == new => scope_name.range,
                _ => scope_name.range.max(old.range),
            };
            diagnostics.push(Diagnostic::new(
                range,
                DiagnosticKind::ModuleError,
                format!("the name `{}` is defined multiple times", name),
            ));
        }
    }

    fn insert_value_type(
        &mut self,
        name: Ustr,
        scope_name: ScopeName,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        self.insert_value(name, scope_name.clone(), diagnostics);
        self.insert_type(name, scope_name, diagnostics);
    }

    fn insert_value(
        &mut self,
        name: Ustr,
        scope_name: ScopeName,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        Self::insert(name, scope_name, self.db, &mut self.values, diagnostics);
    }

    fn insert_type(
        &mut self,
        name: Ustr,
        scope_name: ScopeName,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        Self::insert(name, scope_name, self.db, &mut self.types, diagnostics);
    }
}

impl<'db> ModuleScope<'db> {
    pub fn value_item(&self, name: &Ustr) -> Option<&ir::ModuleDef<'db>> {
        self.values.get(name)
    }

    pub fn type_item(&self, name: &Ustr) -> Option<&ir::ModuleDef<'db>> {
        self.types.get(name)
    }

    pub fn type_scope_names(&self, name: &Ustr) -> Option<&ScopeName> {
        self.scope_types.get(name)
    }

    pub fn value_scope_names(&self, name: &Ustr) -> Option<&ScopeName> {
        self.scope_values.get(name)
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = (&UstrHash, &ir::ModuleDef)> {
        self.values.iter()
    }

    pub fn types(&self) -> impl ExactSizeIterator<Item = (&UstrHash, &ir::ModuleDef)> {
        self.types.iter()
    }

    pub fn global_imports(&self) -> &[ir::Path] {
        &self.global_imports
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
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
    let mut scope_names = ScopeNames::new(db);

    for strct in items.structs(db) {
        let range = strct
            .ast_ptr(db)
            .to_node(&parse.syntax_node(db))
            .syntax()
            .text_range();
        source_map
            .structs
            .insert(MyAstPtr(strct.ast_ptr(db).clone()), *strct);
        scope_names.insert_type(
            strct.name(db),
            ScopeName {
                path: ir::Path(vec![strct.name(db)]),
                range,
                item: Some(ModuleDefKind::Struct),
            },
            &mut scope.diagnostics,
        );
        scope
            .types
            .insert(strct.name(db).into(), ir::ModuleDef::Struct(*strct));
    }

    for enum_item in items.enums(db) {
        let range = enum_item
            .ast_ptr(db)
            .to_node(&parse.syntax_node(db))
            .syntax()
            .text_range();
        // source_map
        //     .structs
        //     .insert(MyAstPtr(enum_item.ast_ptr(db).clone()), *enum_item);
        scope_names.insert_type(
            enum_item.name(db),
            ScopeName {
                path: ir::Path(vec![enum_item.name(db)]),
                range,
                item: Some(ModuleDefKind::Struct),
            },
            &mut scope.diagnostics,
        );
        scope
            .types
            .insert(enum_item.name(db).into(), ir::ModuleDef::Enum(*enum_item));
    }

    for func in items.functions(db) {
        let range = func
            .ast_ptr(db)
            .to_node(&parse.syntax_node(db))
            .syntax()
            .text_range();
        source_map
            .functions
            .insert(MyAstPtr(func.ast_ptr(db).clone()), *func);
        scope_names.insert_value(
            func.name(db),
            ScopeName {
                path: ir::Path(vec![func.name(db)]),
                range,
                item: Some(ModuleDefKind::Function),
            },
            &mut scope.diagnostics,
        );
        scope
            .values
            .insert(func.name(db).into(), ir::ModuleDef::Function(*func));
    }

    for module in items.children(db) {
        let range = module
            .ast_ptr(db)
            .to_node(&parse.syntax_node(db))
            .syntax()
            .text_range();
        let module_name = ide::module_name(db, module.file(db));
        scope_names.insert_type(
            module_name,
            ScopeName {
                path: ir::Path(vec![module_name]),
                range,
                item: Some(ModuleDefKind::Function),
            },
            &mut scope.diagnostics,
        );
        scope
            .types
            .insert(module_name.into(), ir::ModuleDef::Module(module.file(db)));
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
            &mut scope_names,
            &mut scope.global_imports,
            &mut scope.diagnostics,
        );
    }

    scope.scope_values = scope_names.values;
    scope.scope_types = scope_names.types;

    (scope.into(), source_map.into())
}

#[salsa::tracked]
pub fn resolve_imports(db: &dyn salsa::Database, file: ide::File) -> Vec<Diagnostic> {
    let items = lower::module_items(db, file);
    let parse = ide::parse(db, file);
    let mut diagnostics = vec![];
    for import in items.use_imports(db) {
        let Some(use_tree) = import
            .ast_ptr(db)
            .to_node(&parse.syntax_node(db))
            .use_tree()
        else {
            continue;
        };
        resolve_use_tree(db, file, &use_tree, ir::Path(vec![]), &mut diagnostics);
    }
    diagnostics
}

fn resolve_use_tree(
    db: &dyn salsa::Database,
    file: ide::File,
    tree: &ast::UseTree,
    path: ir::Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    match tree {
        ast::UseTree::UseName(use_name) => {
            let mut path = path.clone();
            let name = use_name.name()?.text()?;
            path.0.push(name);

            if resolver::resolve_path(db, file, path).is_none() {
                diagnostics.push(Diagnostic::new(
                    use_name.syntax().text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                ));
            }
        }
        ast::UseTree::UseSelfName(use_self_name) => {
            let mut path = path.clone();
            let name = Ustr::from("self");
            path.0.push(name);

            if resolver::resolve_path(db, file, path).is_none() {
                diagnostics.push(Diagnostic::new(
                    use_self_name.syntax().text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                ));
            }
        }
        ast::UseTree::UsePath(use_path) => {
            let mut path = path.clone();
            let name = use_path.name()?.text()?;
            path.0.push(name);
            if resolver::resolve_path(db, file, path.clone()).is_none() {
                diagnostics.push(Diagnostic::new(
                    use_path.name()?.syntax().text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                ));
            }
            resolve_use_tree(db, file, &use_path.use_tree()?, path, diagnostics)?;
        }
        ast::UseTree::UseRootPath(use_root_path) => {
            let mut path = path.clone();
            let name = "root".into();
            path.0.push(name);
            if resolver::resolve_path(db, file, path.clone()).is_none() {
                diagnostics.push(Diagnostic::new(
                    use_root_path.root_token()?.text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                ))
            }
            resolve_use_tree(db, file, &use_root_path.use_tree()?, path, diagnostics)?;
        }
        ast::UseTree::UseTreeList(use_tree_list) => {
            for elem in use_tree_list.elements() {
                resolve_use_tree(db, file, &elem, path.clone(), diagnostics);
            }
            return None;
        }
        ast::UseTree::UseSuperPath(use_super_path) => {
            let mut path = path.clone();
            let name = "root".into();
            path.0.push(name);
            if resolver::resolve_path(db, file, path.clone()).is_none() {
                diagnostics.push(Diagnostic::new(
                    use_super_path.super_token()?.text_range(),
                    DiagnosticKind::ModuleError,
                    format!("unresolved import `{}`", &name),
                ));
            }
            resolve_use_tree(db, file, &use_super_path.use_tree()?, path, diagnostics)?;
        }
        ast::UseTree::UseGlobal(_) => {}
    };
    Some(())
}

fn traverse_use_tree(
    db: &dyn salsa::Database,
    file: ide::File,
    tree: &ast::UseTree,
    path: &ir::Path,
    names: &mut ScopeNames,
    globals: &mut Vec<ir::Path>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    match tree {
        ast::UseTree::UseName(use_name) => {
            let mut path = path.clone();
            let name = use_name.name()?.text()?;
            path.0.push(name);
            names.insert_value_type(
                use_name.name()?.text()?,
                ScopeName {
                    path,
                    range: use_name.syntax().text_range(),
                    item: None,
                },
                diagnostics,
            );
        }
        ast::UseTree::UseSelfName(self_name) => {
            let path = path.clone();
            let name = *path.0.last().unwrap();
            names.insert_value_type(
                name,
                ScopeName {
                    path,
                    range: self_name.syntax().text_range(),
                    item: None,
                },
                diagnostics,
            );
        }
        ast::UseTree::UsePath(use_path) => {
            let mut path = path.clone();
            path.0.push(use_path.name()?.text()?);
            traverse_use_tree(
                db,
                file,
                &use_path.use_tree()?,
                &path,
                names,
                globals,
                diagnostics,
            )?;
        }
        ast::UseTree::UseTreeList(use_tree_list) => {
            for elem in use_tree_list.elements() {
                traverse_use_tree(db, file, &elem, &path, names, globals, diagnostics);
            }
            return None;
        }
        ast::UseTree::UseRootPath(use_root_path) => {
            let mut path = path.clone();
            path.0.push(Ustr::from("root"));
            traverse_use_tree(
                db,
                file,
                &use_root_path.use_tree()?,
                &path,
                names,
                globals,
                diagnostics,
            )?;
        }
        ast::UseTree::UseSuperPath(use_super_path) => {
            let mut path = ir::Path(vec![]);
            let mut current = file;
            while let Some(parent) = lower::module_parent(db, current) {
                path.0.insert(0, ide::module_name(db, parent));
                current = parent;
            }
            traverse_use_tree(
                db,
                file,
                &use_super_path.use_tree()?,
                &path,
                names,
                globals,
                diagnostics,
            )?;
        }
        ast::UseTree::UseGlobal(_) => {
            globals.push(path.clone());
        }
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
            self.traverse_pattern(*param, root);
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
            ir::Expr::Closure { params, output } => {
                //TODO:
            }
            ir::Expr::Is { expr, pat } => {
                self.traverse_pattern(*pat, scope);
                self.traverse_expr(*expr, scope);
            }
            ir::Expr::IsNot { expr, pat } => {
                self.traverse_pattern(*pat, scope);
                self.traverse_expr(*expr, scope);
            }
        }
    }

    fn traverse_expr_stmts(&mut self, stmts: &[StmtId], scope: ScopeId) {
        for stmt_id in stmts {
            let stmt = self.body.stmt(*stmt_id);
            match stmt {
                ir::Stmt::Let {
                    pat: pattern, expr, ..
                } => {
                    self.traverse_pattern(*pattern, scope);
                    self.traverse_expr(*expr, scope);
                }
                ir::Stmt::Expr { expr, .. } => {
                    self.traverse_expr(*expr, scope);
                }
            }
        }
    }

    fn traverse_pattern(&mut self, pattern_id: PatternId, scope: ScopeId) {
        let pattern = &self.body.pattern(pattern_id);
        match pattern {
            ir::Pattern::Missing => {}
            ir::Pattern::Name(ustr) => {
                self.scopes[scope].entries.push(ScopeEntry {
                    name: *ustr,
                    pattern: pattern_id,
                });
            }
            ir::Pattern::Wildcard => {}
            ir::Pattern::Path(_) => {}
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
