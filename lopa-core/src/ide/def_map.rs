use std::{collections::VecDeque, sync::Arc};

use itertools::Itertools;

use crate::{
    def::{
        Symbol,
        hir::{self, Item, Module, UseItem, UseTree},
    },
    ide::{
        Diagnostic, DiagnosticKind, DiagnosticLocation, ModuleTree, ResolveItem, Root,
        TypeModuleDef, ValueModuleDef,
    },
};

#[derive(salsa::SalsaValue, Clone, Debug, PartialEq, Default)]
pub struct DefMap<'db> {
    pub scopes: indexmap::IndexMap<Module<'db>, Arc<ResolvedModuleScope<'db>>>,
}

impl<'db> DefMap<'db> {
    pub fn scope(&self, module: Module<'db>) -> &ResolvedModuleScope<'db> {
        self.scopes.get(&module).unwrap()
    }
}

#[derive(salsa::SalsaValue, Clone, Debug, PartialEq, Default)]
pub struct ResolvedModuleScope<'db> {
    pub values: indexmap::IndexMap<Symbol, ValueModuleDef<'db>>,
    pub types: indexmap::IndexMap<Symbol, TypeModuleDef<'db>>,
    pub global_imports: Vec<Module<'db>>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'db> ResolvedModuleScope<'db> {
    pub fn resolve_name(&self, name: Symbol) -> Option<ResolveItem<'db>> {
        Some(match (self.types.get(&name), self.values.get(&name)) {
            (None, Some(&value)) => ResolveItem::Value(value),
            (Some(&ty), None) => ResolveItem::Type(ty),
            (Some(&ty), Some(&value)) => ResolveItem::Both { ty, value },
            (None, None) => return None,
        })
    }

    fn insert_value(
        &mut self,
        db: &'db dyn salsa::Database,
        name: Symbol,
        value: ValueModuleDef<'db>,
    ) {
        if let Some(old) = self.values.get(&name) {
            if *old != value {
                self.diagnostics.push(Diagnostic {
                    message: format!(
                        "the name `{}` is defined multiple times",
                        value.name(db).value(db)
                    ),
                    location: value.location(db),
                    kind: DiagnosticKind::ModuleError,
                })
            }
            return;
        }
        self.values.insert(name, value);
    }

    fn insert_type(&mut self, db: &'db dyn salsa::Database, name: Symbol, ty: TypeModuleDef<'db>) {
        if let Some(old) = self.types.get(&name) {
            if *old != ty {
                self.diagnostics.push(Diagnostic {
                    message: format!(
                        "the name `{}` is defined multiple times",
                        ty.name(db).value(db)
                    ),
                    location: ty.location(db),
                    kind: DiagnosticKind::ModuleError,
                })
            }
            return;
        }
        self.types.insert(name, ty);
    }

    fn insert_global_import(&mut self, module: Module<'db>) {
        if !self.global_imports.contains(&module) {
            self.global_imports.push(module);
        }
    }
}

#[derive(salsa::SalsaValue, Clone, Copy, PartialEq, Eq, Debug)]
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
    scopes: indexmap::IndexMap<Module<'db>, ResolvedModuleScope<'db>>,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
enum ScopeEntry<'db> {
    Value(ValueModuleDef<'db>),
    Type(TypeModuleDef<'db>),
}

pub fn visit_module_tree<'db, S>(
    db: &'db dyn salsa::Database,
    this: &mut S,
    root: Root,
    mut f: impl FnMut(&mut S, Module<'db>),
) {
    let root = root.root_module(db).unwrap();
    visit_module(db, this, root, &mut f);
}

fn visit_module<'db, S, F: FnMut(&mut S, Module<'db>)>(
    db: &'db dyn salsa::Database,
    this: &mut S,
    module: Module<'db>,
    f: &mut F,
) {
    f(this, module);
    for item in module.items(db).items(db) {
        if let Item::Module(module) = item {
            visit_module(db, this, *module, f);
        }
    }
}

impl<'db> DefMapCtx<'db> {
    //I call it the first rule of functions,
    //which is, functions should be small.
    //And the second rule: they should be *smaller than that*
    fn def_map(mut self) -> DefMap<'db> {
        self.collect_defs();
        self.resolve_tree();
        self.report_errors();
        DefMap {
            scopes: self
                .scopes
                .into_iter()
                .map(|(k, v)| (k, Arc::new(v)))
                .collect(),
        }
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
                        let Some(items) = self.resolve_name(import.parent, name) else {
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
                            self.scopes
                                .get_mut(&import.module)
                                .unwrap()
                                .diagnostics
                                .push(Diagnostic {
                                    message: "too many leading `super` keywords".to_string(),
                                    location: DiagnosticLocation::UseTree {
                                        use_id: import.use_item.id(self.db),
                                        tree_id: import.tree.id(self.db),
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
                            tree: use_tree,
                            ..import
                        });
                    }
                    hir::UseTreeKind::Name(symbol) => {
                        let Some(entries) = self.resolve_name(import.parent, symbol) else {
                            self.work_list.push_back(import);
                            continue;
                        };

                        match entries.as_slice() {
                            [entry] => {
                                progress = true;
                                match entry {
                                    ScopeEntry::Value(value) => self
                                        .scopes
                                        .get_mut(&import.module)
                                        .unwrap()
                                        .insert_value(self.db, symbol, *value),
                                    ScopeEntry::Type(ty) => self
                                        .scopes
                                        .get_mut(&import.module)
                                        .unwrap()
                                        .insert_type(self.db, symbol, *ty),
                                }
                            }
                            [ScopeEntry::Type(ty), ScopeEntry::Value(value)]
                            | [ScopeEntry::Value(value), ScopeEntry::Type(ty)] => {
                                progress = true;
                                self.scopes
                                    .get_mut(&import.module)
                                    .unwrap()
                                    .insert_value(self.db, symbol, *value);
                                self.scopes
                                    .get_mut(&import.module)
                                    .unwrap()
                                    .insert_type(self.db, symbol, *ty);
                            }
                            _ => {
                                self.work_list.push_back(import);
                                continue;
                            }
                        }
                    }
                    hir::UseTreeKind::Global => {
                        progress = true;
                        self.scopes
                            .get_mut(&import.module)
                            .unwrap()
                            .insert_global_import(import.parent);
                    }
                    hir::UseTreeKind::SelfUse => {
                        progress = true;
                        self.scopes.get_mut(&import.module).unwrap().insert_type(
                            self.db,
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
                        self.scopes
                            .get_mut(&import.module)
                            .unwrap()
                            .diagnostics
                            .push(Diagnostic {
                                message: format!("unresolved import `{}`", name.value(self.db)),
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
                            self.scopes
                                .get_mut(&import.module)
                                .unwrap()
                                .diagnostics
                                .push(Diagnostic {
                                    message: format!("unresolved import `{}`", name.value(self.db)),
                                    location: DiagnosticLocation::UseTree {
                                        use_id: import.use_item.id(self.db),
                                        tree_id: import.tree.id(self.db),
                                    },
                                    kind: DiagnosticKind::ModuleError,
                                });
                        }
                        [_, ..] => {
                            self.scopes
                                .get_mut(&import.module)
                                .unwrap()
                                .diagnostics
                                .push(Diagnostic {
                                    message: format!(
                                        "name `{}` is defined multiple times",
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
                        self.scopes
                            .get_mut(&import.module)
                            .unwrap()
                            .diagnostics
                            .push(Diagnostic {
                                message: format!("unresolved import `{}`", name.value(self.db)),
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
                            self.scopes
                                .get_mut(&import.module)
                                .unwrap()
                                .diagnostics
                                .push(Diagnostic {
                                    message: format!("unresolved import `{}`", name.value(self.db)),
                                    location: DiagnosticLocation::UseTree {
                                        use_id: import.use_item.id(self.db),
                                        tree_id: import.tree.id(self.db),
                                    },
                                    kind: DiagnosticKind::ModuleError,
                                });
                        }
                        [_, ..] => {
                            self.scopes
                                .get_mut(&import.module)
                                .unwrap()
                                .diagnostics
                                .push(Diagnostic {
                                    message: format!(
                                        "name `{}` is defined multiple times",
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
                _ => {}
            }
        }
    }

    fn resolve_name(&self, module: Module<'db>, name: Symbol) -> Option<Vec<ScopeEntry<'db>>> {
        let mut items = vec![];

        let scope = self.scopes.get(&module)?;

        if let Some(value) = scope.values.get(&name) {
            items.push(ScopeEntry::Value(*value))
        }
        if let Some(ty) = scope.types.get(&name) {
            items.push(ScopeEntry::Type(*ty))
        }

        //TODO: resolve cycles
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
        visit_module_tree(self.db, self, self.root, |this, module| {
            let mut scopes = ResolvedModuleScope::default();
            for item in module.items(this.db).items(this.db) {
                match item {
                    Item::Function(item) => {
                        scopes.insert_value(
                            self.db,
                            item.name(this.db),
                            ValueModuleDef::Function(*item),
                        );
                    }
                    Item::Struct(item) => {
                        scopes.insert_type(
                            self.db,
                            item.name(this.db),
                            TypeModuleDef::Struct(*item),
                        );
                    }
                    Item::Class(item) => {
                        scopes.insert_type(
                            self.db,
                            item.name(this.db),
                            TypeModuleDef::Class(*item),
                        );
                    }
                    Item::Trait(item) => {
                        scopes.insert_type(
                            self.db,
                            item.name(this.db),
                            TypeModuleDef::Trait(*item),
                        );
                    }
                    Item::Enum(item) => {
                        scopes.insert_type(self.db, item.name(this.db), TypeModuleDef::Enum(*item));
                    }
                    Item::Module(item) => {
                        scopes.insert_type(
                            self.db,
                            item.name(this.db),
                            TypeModuleDef::Module(*item),
                        );
                    }
                    Item::Use(item) => {
                        if let Some(tree) = item.use_tree(this.db) {
                            this.work_list.push_back(Import {
                                use_item: *item,
                                parent: module,
                                module,
                                tree,
                            });
                        }
                    }
                    Item::Impl(_) => {}
                }
            }
            this.scopes.insert(module, scopes);
        });
    }

    pub fn scope(&self, module: Module<'db>) -> &ResolvedModuleScope<'db> {
        self.scopes.get(&module).unwrap()
    }

    pub fn scope_mut(&mut self, module: Module<'db>) -> &mut ResolvedModuleScope<'db> {
        self.scopes.get_mut(&module).unwrap()
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
            scopes: Default::default(),
        };
        ctx.def_map()
    }
}

#[salsa::tracked]
impl<'db> Module<'db> {
    pub fn resolved_scope(
        self,
        db: &'db dyn salsa::Database,
    ) -> &'db Arc<ResolvedModuleScope<'db>> {
        self.root(db).def_map(db).scopes.get(&self).unwrap()
    }
}

mod test {
    use std::{path::PathBuf, sync::Arc};

    use salsa::Setter;

    use crate::{
        def::hir::Module,
        ide::{self, DefMap, Root, visit_module_tree},
    };

    fn stringify_def_map<'db>(
        db: &'db dyn salsa::Database,
        def_map: &DefMap<'db>,
        root: Root,
    ) -> String {
        struct PrintCtx<'db> {
            db: &'db dyn salsa::Database,
            def_map: &'db DefMap<'db>,
            output: String,
            depth: u32,
            root: Root,
        }

        impl<'db> PrintCtx<'db> {
            fn print_module(&mut self, module: Module<'db>) {
                let scope = self.def_map.scope(module);
                let types = scope.types.iter().map(|(name, ty)| {
                    (
                        name,
                        match ty {
                            ide::TypeModuleDef::Struct(item) => item.path(self.db).as_path(self.db),
                            ide::TypeModuleDef::Class(item) => item.path(self.db).as_path(self.db),
                            ide::TypeModuleDef::Trait(item) => item.path(self.db).as_path(self.db),
                            ide::TypeModuleDef::Enum(item) => item.path(self.db).as_path(self.db),
                            ide::TypeModuleDef::Module(item) => item.path(self.db).as_path(self.db),
                        },
                    )
                });
                let values = scope.values.iter().map(|(name, ty)| {
                    (
                        name,
                        match ty {
                            ide::ValueModuleDef::Function(item) => {
                                item.path(self.db).as_path(self.db)
                            }
                        },
                    )
                });
                self.print_line(&format!("{}: ", module.path(self.db).as_path(self.db)));
                self.add_depth(|this| {
                    if types.len() > 0 {
                        this.print_line("types: ");
                        this.add_depth(|this| {
                            for (name, ty) in types {
                                this.print_line(&format!("{}: {},", name.value(this.db), ty));
                            }
                        });
                    }
                    if values.len() > 0 {
                        this.print_line("values: ");
                        this.add_depth(|this| {
                            for (name, value) in values {
                                this.print_line(&format!("{}: {},", name.value(this.db), value));
                            }
                        });
                    }
                    if !scope.diagnostics.is_empty() {
                        this.print_line("diagnostic: ");
                        this.add_depth(|this| {
                            for diagnostic in scope.diagnostics.iter() {
                                this.print_line(&diagnostic.message);
                            }
                        });
                    }
                });
            }

            fn add_depth(&mut self, f: impl FnOnce(&mut Self)) {
                self.depth += 1;
                f(self);
                self.depth -= 1;
            }

            fn print_line(&mut self, line: &str) {
                (0..self.depth).for_each(|_| self.output.push_str("  "));
                self.output.push_str(line);
                self.output.push('\n');
            }

            fn print_def_map(&mut self) {
                visit_module_tree(self.db, self, self.root, |this, module| {
                    this.print_module(module);
                });
            }
        }

        let mut ctx = PrintCtx {
            db,
            def_map,
            output: String::new(),
            depth: 0,
            root,
        };

        ctx.print_def_map();
        ctx.output
    }

    fn def_map(source: &str) -> String {
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
            stringify_def_map(&db, def_map, root)
        })
    }

    #[test]
    fn items() {
        insta::assert_snapshot!(def_map(
            "
            mod bar {
                pub mod foo {}
            }
            mod baz {
                pub mod foo {}
            }
            use bar::*;
            use baz::*;
            use foo;
            "
        ));
        insta::assert_snapshot!(def_map(
            "
            mod foo {}
            mod foo {}
            "
        ));
        insta::assert_snapshot!(def_map(
            "
            mod foo {}
            use root::foo;
            use root::foo1;
            "
        ));
        insta::assert_snapshot!(def_map(
            "
            struct X {}
            mod inner {
                use super::X;
            }
            "
        ));
        insta::assert_snapshot!(def_map(
            "
            mod foo {
                use super::bar::test;
            }
            mod bar {
                fn test() {}
            }
            "
        ));
    }
    #[test]
    fn temp() {
        def_map(
            "
            mod foo {
                use super::bar::test;
            }
            mod bar {
                fn test() {}
            }
            ",
        );
    }
}
