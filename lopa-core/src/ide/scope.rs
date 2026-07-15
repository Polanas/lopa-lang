use std::{path, sync::Arc};

use notify_rust::Notification;
use salsa::Accumulator;

use crate::{
    def::{
        AstId, Symbol, SymbolList,
        hir::{self, Enum, Function, Item, Module, Struct},
    },
    ide::{self, Diagnostic, DiagnosticKind, DiagnosticLocation, diagnostics},
};

#[derive(salsa::Supertype, Clone, PartialEq, Eq, Hash, Debug, salsa::Update)]
pub enum ModuleDef<'db> {
    Function(Function<'db>),
    Struct(Struct<'db>),
    Enum(Enum<'db>),
    Module(Module<'db>),
}

#[derive(Clone, PartialEq, Default, salsa::Update)]
pub struct ModuleScope<'db> {
    pub values: indexmap::IndexMap<Symbol, ModuleDef<'db>>,
    pub types: indexmap::IndexMap<Symbol, ModuleDef<'db>>,

    pub visible_values: indexmap::IndexMap<Symbol, ScopeName>,
    pub visible_types: indexmap::IndexMap<Symbol, ScopeName>,

    pub global_imports: Vec<SymbolList>,
}

impl<'db> ModuleScope<'db> {
    pub fn value_item(&self, name: Symbol) -> Option<&ModuleDef<'db>> {
        self.values.get(&name)
    }

    pub fn type_item(&self, name: Symbol) -> Option<&ModuleDef<'db>> {
        self.types.get(&name)
    }

    pub fn visible_value(&self, name: Symbol) -> Option<&ScopeName> {
        self.visible_values.get(&name)
    }

    pub fn visible_type(&self, name: Symbol) -> Option<&ScopeName> {
        self.visible_types.get(&name)
    }

    pub fn global_imports(&self) -> &[SymbolList] {
        &self.global_imports
    }

    pub fn visible_types(&self) -> &indexmap::IndexMap<Symbol, ScopeName> {
        &self.visible_types
    }
}

#[salsa::interned(debug, no_lifetime)]
pub struct ScopeName {
    pub path: SymbolList,
    pub location: DiagnosticLocation,
}

struct ScopeNames<'db> {
    db: &'db dyn salsa::Database,
    values: indexmap::IndexMap<Symbol, ScopeName>,
    types: indexmap::IndexMap<Symbol, ScopeName>,
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
        scope_name: ScopeName,
        db: &dyn salsa::Database,
        names: &mut indexmap::IndexMap<Symbol, ScopeName>,
    ) {
        //TODO: try to choose order based on id number (id bigger -> item lower)
        if let Some(old) = names.insert(name, scope_name) {
            Diagnostic {
                message: format!("the name `{}` is defined multiple times", name.value(db)),
                location: scope_name.location(db),
                kind: DiagnosticKind::ModuleError,
            }
            .accumulate(db);
        }
    }

    fn insert_value_type(&mut self, name: Symbol, scope_name: ScopeName) {
        self.insert_value(name, scope_name);
        self.insert_type(name, scope_name);
    }

    fn insert_value(&mut self, name: Symbol, scope_name: ScopeName) {
        Self::insert(name, scope_name, self.db, &mut self.values);
    }

    fn insert_type(&mut self, name: Symbol, scope_name: ScopeName) {
        Self::insert(name, scope_name, self.db, &mut self.types);
    }
}

#[salsa::tracked(returns(ref))]
pub fn module_scope<'db>(
    db: &'db dyn salsa::Database,
    module: Module<'db>,
) -> Arc<ModuleScope<'db>> {
    let items = module.items(db).items(db);

    let mut scope = ModuleScope::default();
    let mut scope_names = ScopeNames::new(db);

    if let Some(parent) = module.parent(db)
        && matches!(module.kind(db), hir::ModuleKind::Definition { .. })
    {
        let parent_scope = module_scope(db, parent);
        for (k, v) in parent_scope.visible_values.iter() {
            scope_names.values.insert(*k, *v);
        }
        for (k, v) in parent_scope.visible_types.iter() {
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
                );
                scope.types.insert(item.name(db), ModuleDef::Struct(*item));
            }
            Item::Function(item) => {
                scope_names.insert_value(
                    item.name(db),
                    ScopeName::new(
                        db,
                        SymbolList::new(db, [item.name(db)]),
                        DiagnosticLocation::Function(item.id(db)),
                    ),
                );
                scope
                    .values
                    .insert(item.name(db), ModuleDef::Function(*item));
            }
            Item::Enum(item) => {
                scope_names.insert_type(
                    item.name(db),
                    ScopeName::new(
                        db,
                        SymbolList::new(db, [item.name(db)]),
                        DiagnosticLocation::Enum(item.id(db)),
                    ),
                );
                scope.types.insert(item.name(db), ModuleDef::Enum(*item));
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
                    );
                }
                scope.types.insert(item.name(db), ModuleDef::Module(*item));
            }
            Item::Use(item) => {
                let mut traverse_ctx = TraversUseTree {
                    db,
                    use_item: *item,
                    names: &mut scope_names,
                    module,
                    global_imports: &mut scope.global_imports,
                };
                if let Some(use_tree) = item.use_tree(db) {
                    traverse_ctx.traverse(use_tree, SymbolList::new(db, []));
                }
            }
            Item::Impl(_) => {}
        }
    }

    scope.visible_values = scope_names.values;
    scope.visible_types = scope_names.types;

    scope.into()
}

struct TraversUseTree<'db, 'a> {
    db: &'db dyn salsa::Database,
    use_item: hir::UseItem<'db>,
    module: hir::Module<'db>,
    names: &'a mut ScopeNames<'db>,
    global_imports: &'a mut Vec<SymbolList>,
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
                self.traverse(use_tree, path)?;
            }
            hir::UseTreeKind::Super {
                use_tree: super_use_tree,
            } => {
                let Some(parent) = self.module.parent(self.db) else {
                    Diagnostic {
                        message: "too many leading `super` keywords".to_string(),
                        location: DiagnosticLocation::UseTree {
                            use_id: self.use_item.id(self.db),
                            tree_id: use_tree.id(self.db),
                        },
                        kind: DiagnosticKind::ModuleError,
                    }
                    .accumulate(self.db);
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
