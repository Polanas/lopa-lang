use std::collections::VecDeque;

use itertools::Itertools;

use crate::{
    common::PathNodeType,
    def::{
        Symbol, SymbolList,
        hir::{self, Enum, Function, HirNode, Item, Module, Struct, UseItem, UseTree},
    },
    hir_walker::{HirWalker, walk_expr, walk_pat, walk_type_expr},
    ide::{
        Diagnostic, DiagnosticKind, DiagnosticLocation, ModuleTree, Root, ScopePath, TypeModuleDef,
        ValueModuleDef,
    },
};

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub enum ResolveItem<'db> {
    Type(TypeModuleDef<'db>),
    Value(ValueModuleDef<'db>),
    Both {
        ty: TypeModuleDef<'db>,
        value: ValueModuleDef<'db>,
    },
}

#[salsa::tracked]
impl<'db> Struct<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.walk_item(db, hir::Item::Struct(self), &mut ());
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> Module<'db> {
    pub fn path_map(self, db: &'db dyn salsa::Database) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, self);
        ctx.collect_global_imports();
        ctx.walk_item(db, hir::Item::Module(self), &mut ());
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> Enum<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.walk_item(db, hir::Item::Enum(self), &mut ());
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.walk_item(db, hir::Item::Function(self), &mut ());
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> UseItem<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.resolve_use_item(self);
        ctx.path_map()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub struct PathMap<'db> {
    pub items: indexmap::IndexMap<SymbolList, ResolveItem<'db>>,
    pub diagnostics: Vec<Diagnostic>,
}

struct ResolveUseTree<'db, 'a> {
    resolve_ctx: &'a mut PathResolveCtx<'db>,
    use_item: UseItem<'db>,
    module: Module<'db>,
}

impl<'db, 'a> std::ops::Deref for ResolveUseTree<'db, 'a> {
    type Target = PathResolveCtx<'db>;

    fn deref(&self) -> &Self::Target {
        self.resolve_ctx
    }
}

impl<'db, 'a> std::ops::DerefMut for ResolveUseTree<'db, 'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.resolve_ctx
    }
}

impl<'db, 'a> ResolveUseTree<'db, 'a> {
    fn resolve(&mut self) -> Option<()> {
        self.resolve_inner(
            self.use_item.use_tree(self.db)?,
            SymbolList::new(self.resolve_ctx.db, []),
        )
    }
    fn resolve_inner(&mut self, use_tree: hir::UseTree<'db>, path: SymbolList) -> Option<()> {
        let use_item = self.use_item;
        match use_tree.kind(self.db) {
            hir::UseTreeKind::Name(name) => {
                let path = path.push(self.db, name);
                self.insert_use_tree_path(path, use_item, use_tree);
            }
            hir::UseTreeKind::Path {
                name,
                use_tree: path_use_tree,
            } => {
                let path = path.push(self.db, name);
                self.insert_use_tree_path(path, use_item, use_tree)?;
                self.resolve_inner(path_use_tree, path)?;
            }
            //NOTE: super can only be used in start positions, so foo::bar::super is not valid
            hir::UseTreeKind::Super { use_tree } => {
                let parent = self.module.parent(self.db).unwrap();
                self.module = parent;
                let path = parent.absolute_path(self.db);
                self.resolve_inner(use_tree, path)?;
            }
            hir::UseTreeKind::Root { use_tree } => {
                let path = SymbolList::new(self.db, [Symbol::new(self.db, "root")]);
                self.resolve_inner(use_tree, path)?;
            }
            hir::UseTreeKind::TreeList(use_tree_list) => {
                for item in use_tree_list.items(self.db).iter() {
                    self.resolve_inner(*item, path);
                }
            }
            hir::UseTreeKind::SelfUse => {
                self.insert_use_tree_path(path, use_item, use_tree)?;
            }
            hir::UseTreeKind::Global => {}
        }
        Some(())
    }
}

struct PathSegmentDiagnostic {
    offset: usize,
    message: String,
}

struct PathResolveCtx<'db> {
    db: &'db dyn salsa::Database,
    module: Module<'db>,
    diagnostics: Vec<Diagnostic>,
    items: indexmap::IndexMap<SymbolList, ResolveItem<'db>>,
    global_import_modules: Vec<Module<'db>>,
}

impl<'db> HirWalker<'db> for PathResolveCtx<'db> {
    type Context = ();
    fn walk_expr(&mut self, db: &'db dyn salsa::Database, expr: hir::Expr<'db>, ctx: &mut ()) {
        match expr.kind(db) {
            hir::ExprKind::Path(path) => {
                self.insert_path(path.as_symbol_list(db), expr, PathNodeType::Expr);
            }
            hir::ExprKind::Record { path, .. } => {
                self.insert_path(path.as_symbol_list(self.db), expr, PathNodeType::Record);
            }
            _ => {}
        }
        walk_expr(db, self, expr, ctx);
    }

    fn walk_type_expr(
        &mut self,
        db: &'db dyn salsa::Database,
        type_expr: hir::TypeExpr<'db>,
        ctx: &mut (),
    ) {
        if let hir::TypeExprKind::Path(path) = type_expr.kind(db) {
            self.insert_path(path.as_symbol_list(db), type_expr, PathNodeType::Type);
        }
        walk_type_expr(db, self, type_expr, ctx);
    }

    fn walk_pat(&mut self, db: &'db dyn salsa::Database, pat: hir::Pat<'db>, ctx: &mut ()) {
        if let hir::PatKind::Path(path) = pat.kind(db) {
            self.insert_path(path.as_symbol_list(db), pat, PathNodeType::Pat);
        }
        walk_pat(db, self, pat, ctx);
    }
}

impl<'db> PathResolveCtx<'db> {
    fn new(db: &'db dyn salsa::Database, module: Module<'db>) -> Self {
        Self {
            db,
            module,
            diagnostics: Default::default(),
            items: Default::default(),
            global_import_modules: Default::default(),
        }
    }

    fn collect_global_imports(&mut self) {
        // let scope = self.module.scope_names(self.db);
        // for global in scope.global_imports.iter() {
        //     if let Ok(ResolveItem::Type(ty) | ResolveItem::Both { ty, .. }) =
        //         self.resolve_path(*global)
        //         && let TypeModuleDef::Module(module) = ty
        //     {
        //         self.global_import_modules.push(module);
        //     }
        // }
    }

    fn canonicalize(&mut self, path: SymbolList) -> SymbolList {
        let scope = self.module.scope_names(self.db);
        let defs = self.module.defs(self.db);
        let mut path_vec = path.symbols(self.db).to_vec();
        loop {
            let first = path_vec[0];
            if defs.types.contains_key(&first) {
                break;
            }

            let Some(scope_name) = scope.types.get(&first) else {
                return path;
            };

            let symbols = scope_name
                .path(self.db)
                .symbols(self.db)
                .iter()
                .rev()
                .skip(1);
            if symbols.len() == 0 {
                break;
            }
            for symbol in symbols {
                path_vec.insert(0, *symbol);
            }
        }

        SymbolList::new(self.db, &path_vec)
    }

    fn insert_path<H: HirNode<'db>>(&mut self, path: SymbolList, node: H, node_type: PathNodeType) {
        match self.resolve_path(path) {
            Ok(item) => {
                self.items.insert(path, item);
            }
            Err(err) => {
                // salsa::attach(self.db, || {
                //     panic!("{:?} {:?}", path, node_type);
                // });
                self.diagnostics.push(Diagnostic {
                    message: err.message,
                    location: DiagnosticLocation::PathSegment {
                        id: node.hir_id(self.db),
                        source: node.body_map_source(self.db).body_map(self.db),
                        offset: err.offset,
                        node_type,
                    },
                    kind: DiagnosticKind::TypeError,
                });
            }
        }
    }

    fn resolve_path(
        &mut self,
        path: SymbolList,
    ) -> Result<ResolveItem<'db>, PathSegmentDiagnostic> {
        let defs = self.module.defs(self.db);
        let scope_names = self.module.scope_names(self.db);
        todo!()
    }

    // fn resolve_path(
    //     &mut self,
    //     path: SymbolList,
    // ) -> Result<ResolveItem<'db>, PathSegmentDiagnostic> {
    //     let canonicalized = self.canonicalize(path);
    //     let mut symbols = canonicalized.symbols(self.db).iter().enumerate();
    //     let mut current_item = match symbols.next().unwrap().1.value(self.db) {
    //         "root" => ResolveItem::Type(TypeModuleDef::Module(
    //             self.module.root(self.db).root_module(self.db).unwrap(),
    //         )),
    //         other => {
    //             let other = Symbol::new(self.db, other);
    //             let Some(item) = self.module.defs(self.db).resolve_item(other).or_else(|| {
    //                 for module in self.global_import_modules.iter() {
    //                     if let Some(item) = module.defs(self.db).resolve_item(other) {
    //                         return Some(item);
    //                     }
    //                 }
    //                 None
    //             }) else {
    //                 return Err(PathSegmentDiagnostic {
    //                     offset: 0,
    //                     message: format!("could not find `{}` in this scope", other.value(self.db)),
    //                 });
    //             };
    //             item
    //         }
    //     };
    //
    //     for (id, symbol) in symbols {
    //         match current_item {
    //             ResolveItem::Type(ty) | ResolveItem::Both { ty, .. } => match ty {
    //                 TypeModuleDef::Struct(_)
    //                 | TypeModuleDef::Enum(_)
    //                 | TypeModuleDef::Class(_)
    //                 | TypeModuleDef::Trait(_) => {
    //                     return Err(PathSegmentDiagnostic {
    //                         offset: id,
    //                         message: "accessing this item not availible yet".to_string(),
    //                     });
    //                 }
    //                 TypeModuleDef::Module(item) => {
    //                     if symbol.value(self.db) == "super" {
    //                         current_item = ResolveItem::Type(TypeModuleDef::Module(
    //                             item.parent(self.db).unwrap(),
    //                         ));
    //                         continue;
    //                     }
    //                     let Some(item) = item.defs(self.db).resolve_item(*symbol) else {
    //                         return Err(PathSegmentDiagnostic {
    //                             offset: id,
    //                             message: format!(
    //                                 "could not find `{}` in this scope",
    //                                 symbol.value(self.db)
    //                             ),
    //                         });
    //                     };
    //                     current_item = item;
    //                 }
    //             },
    //             ResolveItem::Value(value) => match value {
    //                 ValueModuleDef::Function(function) => {
    //                     return Err(PathSegmentDiagnostic {
    //                         offset: id,
    //                         message: format!(
    //                             "could not find access members of function `{}`",
    //                             function.name(self.db).value(self.db)
    //                         ),
    //                     });
    //                 }
    //             },
    //         }
    //     }
    //
    //     Ok(current_item)
    // }

    fn insert_use_tree_path(
        &mut self,
        path: SymbolList,
        use_item: hir::UseItem<'db>,
        use_tree: hir::UseTree<'db>,
    ) -> Option<()> {
        match self.resolve_path(path) {
            Ok(item) => {
                self.items.insert(path, item);
                Some(())
            }
            Err(err) => {
                self.diagnostics.push(Diagnostic {
                    message: err.message,
                    location: DiagnosticLocation::UseTree {
                        use_id: use_item.id(self.db),
                        tree_id: use_tree.id(self.db),
                    },
                    kind: DiagnosticKind::TypeError,
                });
                None
            }
        }
    }

    fn resolve_use_item(&mut self, use_item: hir::UseItem<'db>) {
        let module = self.module;
        ResolveUseTree {
            resolve_ctx: self,
            use_item,
            module,
        }
        .resolve();
    }

    fn path_map(self) -> PathMap<'db> {
        PathMap {
            items: self.items,
            diagnostics: self.diagnostics,
        }
    }
}

#[derive(salsa::SalsaValue, Clone, Debug, PartialEq, Default)]
pub struct DefMap<'db> {
    pub scopes: indexmap::IndexMap<Module<'db>, ResolvedModuleScope<'db>>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'db> DefMap<'db> {
    pub fn module_scope(&self, module: Module<'db>) -> &ResolvedModuleScope<'db> {
        self.scopes.get(&module).unwrap()
    }

    pub fn module_scope_mut(&mut self, module: Module<'db>) -> &mut ResolvedModuleScope<'db> {
        self.scopes.get_mut(&module).unwrap()
    }
}

#[derive(salsa::SalsaValue, Clone, Debug, PartialEq, Default)]
pub struct ResolvedModuleScope<'db> {
    pub values: indexmap::IndexMap<Symbol, Vec<ValueModuleDef<'db>>>,
    pub types: indexmap::IndexMap<Symbol, Vec<TypeModuleDef<'db>>>,
    pub global_imports: Vec<Module<'db>>,
}

impl<'db> ResolvedModuleScope<'db> {
    fn insert_value(&mut self, name: Symbol, value: ValueModuleDef<'db>) {
        let values = self.values.entry(name).or_default();
        if !values.contains(&value) {
            values.push(value);
        }
    }

    fn insert_type(&mut self, name: Symbol, ty: TypeModuleDef<'db>) {
        let types = self.types.entry(name).or_default();
        if !types.contains(&ty) {
            types.push(ty);
        }
    }

    fn insert_global_import(&mut self, module: Module<'db>) {
        if !self.global_imports.contains(&module) {
            self.global_imports.push(module);
        }
    }
}

#[derive(salsa::SalsaValue, Clone, Copy, PartialEq, Eq)]
struct Import<'db> {
    module: Module<'db>,
    parent: Module<'db>,
    tree: UseTree<'db>,
    use_item: UseItem<'db>,
}

pub struct DefMapCtx<'db> {
    def_map: DefMap<'db>,
    mod_tree: &'db ModuleTree<'db>,
    root: Root,
    db: &'db dyn salsa::Database,
    work_list: VecDeque<Import<'db>>,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
enum ScopeEntry<'db> {
    Value(ValueModuleDef<'db>),
    Type(TypeModuleDef<'db>),
}

impl<'db> DefMapCtx<'db> {
    //I call it the first rule of functions,
    //which is, functions should be small.
    //And the second rule: they should be *smaller than that*
    fn def_map(mut self) -> DefMap<'db> {
        self.collect_defs();
        self.resolve_tree();
        self.report_errors();
        self.def_map
    }

    fn resolve_tree(&mut self) {
        loop {
            let mut old_work_list = std::mem::take(&mut self.work_list);
            let mut progress = false;
            while let Some(import) = old_work_list.pop_front() {
                match import.tree.kind(self.db) {
                    hir::UseTreeKind::TreeList(list) => {
                        for &tree in list.items(self.db) {
                            progress = true;
                            self.work_list.push_back(Import { tree, ..import });
                        }
                    }
                    hir::UseTreeKind::Path { name, use_tree } => {
                        let Some(items) = self.resolve_name(import.module, name) else {
                            self.work_list.push_back(import);
                            continue;
                        };
                        let modules = items
                            .into_iter()
                            .filter_map(|i| match i {
                                ScopeEntry::Type(TypeModuleDef::Module(m)) => Some(m),
                                _ => None,
                            })
                            .collect_vec();

                        match modules.as_slice() {
                            [module] => {
                                progress = true;
                                self.work_list.push_back(Import {
                                    parent: *module,
                                    tree: use_tree,
                                    ..import
                                });
                            }
                            [_, ..] | [] => {
                                self.work_list.push_back(import);
                                continue;
                            }
                        }
                    }
                    hir::UseTreeKind::Super { use_tree } => {
                        let Some(parent) = import.module.parent(self.db) else {
                            self.def_map.diagnostics.push(Diagnostic {
                                message: "too many leading `super` keywords".to_string(),
                                location: DiagnosticLocation::UseTree {
                                    use_id: import.use_item.id(self.db),
                                    tree_id: use_tree.id(self.db),
                                },
                                kind: DiagnosticKind::ModuleError,
                            });
                            continue;
                        };
                        progress = true;
                        self.work_list.push_back(Import {
                            parent,
                            tree: use_tree,
                            ..import
                        });
                    }
                    hir::UseTreeKind::Root { use_tree } => {
                        let Some(root_module) = self.root.root_module(self.db) else {
                            continue;
                        };
                        progress = true;
                        self.work_list.push_back(Import {
                            parent: root_module,
                            module: root_module,
                            tree: use_tree,
                            use_item: import.use_item,
                        });
                    }
                    hir::UseTreeKind::Name(symbol) => {
                        let Some(entries) = self.resolve_name(import.parent, symbol) else {
                            self.work_list.push_back(import);
                            continue;
                        };

                        let scope = self.def_map.module_scope_mut(import.module);
                        match entries.as_slice() {
                            [entry] => {
                                progress = true;
                                match entry {
                                    ScopeEntry::Value(value) => scope.insert_value(symbol, *value),
                                    ScopeEntry::Type(ty) => scope.insert_type(symbol, *ty),
                                }
                            }
                            [ScopeEntry::Type(ty), ScopeEntry::Value(value)]
                            | [ScopeEntry::Value(value), ScopeEntry::Type(ty)] => {
                                progress = true;
                                scope.insert_value(symbol, *value);
                                scope.insert_type(symbol, *ty);
                            }
                            _ => {
                                self.work_list.push_back(import);
                                continue;
                            }
                        }
                    }
                    hir::UseTreeKind::Global => {
                        progress = true;
                        self.def_map
                            .module_scope_mut(import.module)
                            .insert_global_import(import.parent);
                    }
                    hir::UseTreeKind::SelfUse => {
                        progress = true;
                        self.def_map.module_scope_mut(import.module).insert_type(
                            import.parent.name(self.db),
                            TypeModuleDef::Module(import.parent),
                        );
                    }
                }
            }
            if !progress {
                break;
            }
        }
    }

    fn report_errors(&mut self) {
        while let Some(import) = self.work_list.pop_front() {
            match import.tree.kind(self.db) {
                hir::UseTreeKind::Path { name, .. } => {
                    let Some(items) = self.resolve_name(import.module, name) else {
                        self.def_map.diagnostics.push(Diagnostic {
                            message: format!("unresolved import {}", name.value(self.db)),
                            location: DiagnosticLocation::UseTree {
                                use_id: import.use_item.id(self.db),
                                tree_id: import.tree.id(self.db),
                            },
                            kind: DiagnosticKind::ModuleError,
                        });
                        continue;
                    };
                    let modules = items
                        .into_iter()
                        .filter_map(|i| match i {
                            ScopeEntry::Type(TypeModuleDef::Module(m)) => Some(m),
                            _ => None,
                        })
                        .collect_vec();
                    match modules.as_slice() {
                        [] => {
                            self.def_map.diagnostics.push(Diagnostic {
                                message: format!("unresolved import {}", name.value(self.db)),
                                location: DiagnosticLocation::UseTree {
                                    use_id: import.use_item.id(self.db),
                                    tree_id: import.tree.id(self.db),
                                },
                                kind: DiagnosticKind::ModuleError,
                            });
                        }
                        [_, ..] => {
                            self.def_map.diagnostics.push(Diagnostic {
                                message: format!(
                                    "name {} is defined multiple times",
                                    name.value(self.db)
                                ),
                                location: DiagnosticLocation::UseTree {
                                    use_id: import.use_item.id(self.db),
                                    tree_id: import.tree.id(self.db),
                                },
                                kind: DiagnosticKind::ModuleError,
                            });
                        }
                    }
                }
                hir::UseTreeKind::Name(name) => {
                    let Some(entries) = self.resolve_name(import.module, name) else {
                        self.def_map.diagnostics.push(Diagnostic {
                            message: format!("unresolved import {}", name.value(self.db)),
                            location: DiagnosticLocation::UseTree {
                                use_id: import.use_item.id(self.db),
                                tree_id: import.tree.id(self.db),
                            },
                            kind: DiagnosticKind::ModuleError,
                        });
                        continue;
                    };
                    match entries.as_slice() {
                        [] => {
                            self.def_map.diagnostics.push(Diagnostic {
                                message: format!("unresolved import {}", name.value(self.db)),
                                location: DiagnosticLocation::UseTree {
                                    use_id: import.use_item.id(self.db),
                                    tree_id: import.tree.id(self.db),
                                },
                                kind: DiagnosticKind::ModuleError,
                            });
                        }
                        [_, ..] => {
                            self.def_map.diagnostics.push(Diagnostic {
                                message: format!(
                                    "name {} is defined multiple times",
                                    name.value(self.db)
                                ),
                                location: DiagnosticLocation::UseTree {
                                    use_id: import.use_item.id(self.db),
                                    tree_id: import.tree.id(self.db),
                                },
                                kind: DiagnosticKind::ModuleError,
                            });
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn resolve_name(&self, module: Module<'db>, name: Symbol) -> Option<Vec<ScopeEntry<'db>>> {
        let mut items = vec![];

        let scope = self.def_map.scopes.get(&module)?;

        if let Some(values) = scope.values.get(&name) {
            values
                .iter()
                .for_each(|v| items.push(ScopeEntry::Value(*v)));
        }
        if let Some(types) = scope.types.get(&name) {
            types.iter().for_each(|v| items.push(ScopeEntry::Type(*v)));
        }

        for global_import in scope.global_imports.iter() {
            items.extend(
                self.resolve_name(*global_import, name)
                    .into_iter()
                    .flatten(),
            );
        }

        if items.is_empty() { None } else { Some(items) }
    }

    fn collect_defs(&mut self) {
        self.visit_module_tree(|this, module| {
            let mut scopes = ResolvedModuleScope::default();
            for item in module.items(this.db).items(this.db) {
                match item {
                    Item::Function(item) => {
                        scopes.insert_value(item.name(this.db), ValueModuleDef::Function(*item));
                    }
                    Item::Struct(item) => {
                        scopes.insert_type(item.name(this.db), TypeModuleDef::Struct(*item));
                    }
                    Item::Class(item) => {
                        scopes.insert_type(item.name(this.db), TypeModuleDef::Class(*item));
                    }
                    Item::Trait(item) => {
                        scopes.insert_type(item.name(this.db), TypeModuleDef::Trait(*item));
                    }
                    Item::Enum(item) => {
                        scopes.insert_type(item.name(this.db), TypeModuleDef::Enum(*item));
                    }
                    Item::Module(item) => {
                        scopes.insert_type(item.name(this.db), TypeModuleDef::Module(*item));
                    }
                    Item::Use(item) => {
                        if let Some(tree) = item.use_tree(this.db) {
                            this.work_list.push_back(Import {
                                module,
                                use_item: *item,
                                tree,
                                parent: module,
                            });
                        }
                    }
                    Item::Impl(_) => {}
                }
            }
            this.def_map.scopes.insert(module, scopes);
        });
    }

    fn visit_module_tree(&mut self, mut f: impl FnMut(&mut Self, Module<'db>)) {
        let root = self.root.root_module(self.db).unwrap();
        self.visit_module(root, &mut f);
    }

    fn visit_module<F: FnMut(&mut Self, Module<'db>)>(&mut self, module: Module<'db>, f: &mut F) {
        f(self, module);
        for item in module.items(self.db).items(self.db) {
            if let Item::Module(module) = item {
                self.visit_module(*module, f);
            }
        }
    }
}

#[salsa::tracked]
impl<'db> Root {
    #[salsa::tracked(returns(ref))]
    pub fn def_map(self, db: &'db dyn salsa::Database) -> DefMap<'db> {
        let ctx = DefMapCtx {
            def_map: DefMap::default(),
            mod_tree: self.module_tree(db),
            work_list: Default::default(),
            db,
            root: self,
        };
        ctx.def_map()
    }
}

#[cfg(test)]
mod test {
    use std::{path::PathBuf, sync::Arc};

    use salsa::Setter;

    use crate::{
        def::hir::{self, Function, Module},
        ide::{self, DefMap},
    };

    fn print_def_map<'db>(db: &'db dyn salsa::Database, def_map: DefMap<'db>) {
        struct PrintCtx<'db> {
            db: &'db dyn salsa::Database,
            def_map: DefMap<'db>,
            output: String,
            depth: u32,
        }

        impl<'db> PrintCtx<'db> {
            fn print_module(&mut self, module: Module<'db>) {
                let scope = self.def_map.module_scope(module);
                for types in 
            }
        }
        // fn print_module<'db>(db: &'db dyn salsa::Database, def_map: DefMap<'db> {}
        // salsa::attach(db, || {
        //
        // });
    }

    #[test]
    fn def_map() {
        let source = "
            mod bar {
                pub mod foo {}
            }
            use bar::*;
            use foo;
        ";
        let mut db = salsa::DatabaseImpl::default();
        let root = ide::Root::new(&db, vec![], PathBuf::from("test"));

        let file = ide::File::new(
            &db,
            Arc::from(source),
            PathBuf::from("test/src/main.lopa"),
            root,
        );

        root.set_files(&mut db).to(vec![file]);
        salsa::attach(&db, || {
            let def_map = root.def_map(&db);
            dbg!(def_map);
        });
    }

    fn assert_module<'db>(db: &'db dyn salsa::Database, module: Module<'db>) {
        let path_map = module.path_map(db);
        if !path_map.diagnostics.is_empty() {
            panic!("{:#?}", path_map.diagnostics);
        }
        for item in module.items(db).items(db) {
            match item {
                hir::Item::Function(item) => assert_function(db, *item, module),
                hir::Item::Module(item) => assert_module(db, *item),
                hir::Item::Impl(item) => {
                    item.contents(db);
                }
                _ => {}
            }
        }
    }

    fn assert_function<'db>(
        db: &'db dyn salsa::Database,
        func: Function<'db>,
        module: Module<'db>,
    ) {
        let path_map = func.path_map(db, module);
        if !path_map.diagnostics.is_empty() {
            panic!("{:#?}", path_map.diagnostics);
        }
    }

    fn assert_resolve_diagnostics(source: &str) {
        let mut db = salsa::DatabaseImpl::default();
        let root = ide::Root::new(&db, vec![], PathBuf::from("test"));
        let file = ide::File::new(
            &db,
            Arc::from(source),
            PathBuf::from("test/src/main.lopa"),
            root,
        );

        root.set_files(&mut db).to(vec![file]);

        if let Some(item) = file.module(&db) {
            assert_module(&db, item);
        }
    }

    #[test]
    fn inner_mod() {
        assert_resolve_diagnostics(
            "mod test {
                mod bar {
                    struct X {}
                    enum Y {}
                }
                use bar::X;
                use bar::Y;
        }",
        );
    }

    #[test]
    fn single_super() {
        assert_resolve_diagnostics(
            "mod test {
                struct X {}
                mod foo {
                    use super::X;
                }
        }",
        );
    }

    #[test]
    fn multi_super() {
        assert_resolve_diagnostics(
            "mod test {
                struct X {}
                mod foo {
                    mod bar {
                        use super::super::X;
                    }
                }
        }",
        );
    }

    #[test]
    fn inner_items() {
        assert_resolve_diagnostics(
            "mod test {
                mod foo {
                    struct X {
                        idk: struct Y {};
                    }
                }
                use foo::X;
                use foo::Y;

                impl X for Y {
                    fn a(){}
                    fn b() {}
                }
        }",
        )
    }

    #[test]
    fn global_imports() {
        assert_resolve_diagnostics(
            "mod test {
                mod foo {
                    mod bar {
                        struct X {
                            idk: struct Y {};
                        }
                    }
                }

                use bar::Y;
                use foo::*;
        }",
        );
    }

    #[test]
    fn fn_exprs() {
        assert_resolve_diagnostics(
            "mod test {
                mod A {
                    mod B {
                        struct C {}
                    }
                }

                fn my_func() {
                }
        }",
        );
    }
}
// let _ = (
//     A::B::C,
//     A::B::C,
//     { if A::B::C {
//                 } {} }
// );

/*
Name resolution notes.

How I imagine it should work:
1. Collect defs (values + types) for module
2. Collect scope names per module:
    Scope names are all names accessible through a module, including `use` statements.
3. Find a local scope of a module, derived from 2: IndexMap<Symbol, Item>
4. Collect all paths that are referenced in a module, and check if they are valid (via PathMap)

Main difficulty lies in 3., as it seems like this step already requried name resolution, which should only happen at 4.
Solution: reuse PathMap for 3. and 4.?


*/
