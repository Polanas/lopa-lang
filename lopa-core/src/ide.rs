mod diagnostics;
mod resolve;
mod scope;

pub use diagnostics::{
    Diagnostic, DiagnosticKind, DiagnosticLocation, RenderedDiagnostic, Severity,
};
use notify_rust::Notification;
pub use resolve::*;
pub use scope::*;

use itertools::Itertools;
use salsa::Accumulator;

use crate::{
    def::{
        self, Symbol, SymbolList,
        hir::{self},
    },
    parsing::{self, AstNode},
};
use std::{fmt::Debug, path::PathBuf, sync::Arc};

#[salsa::input]
pub struct Root {
    #[returns(ref)]
    pub files: Vec<File>,
    #[returns(ref)]
    pub root_dir: PathBuf,
}

impl std::fmt::Debug for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Root").field(&self.0).finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq, salsa::SalsaValue)]
pub struct ModuleTree<'db> {
    pub parents: indexmap::IndexMap<hir::Module<'db>, hir::Module<'db>>,
    pub children: indexmap::IndexMap<hir::Module<'db>, Arc<Vec<hir::Module<'db>>>>,
    pub modules_by_files: indexmap::IndexMap<File, hir::Module<'db>>,
    pub files_by_modules: indexmap::IndexMap<hir::Module<'db>, File>,
    pub diagnostics: Vec<Diagnostic>,
}

#[salsa::tracked]
impl<'db> Root {
    #[salsa::tracked(returns(ref))]
    pub fn module_tree(self, db: &'db dyn salsa::Database) -> ModuleTree<'db> {
        let files_by_names = self.files_by_names(db);
        fn traverse_module<'db>(
            db: &'db dyn salsa::Database,
            module: hir::Module<'db>,
            mut module_dir_path: PathBuf,
            tree: &mut ModuleTree<'db>,
            files_by_names: &indexmap::IndexMap<PathBuf, File>,
        ) {
            match module.kind(db) {
                hir::ModuleKind::Declaration => {
                    let mut file_path = module_dir_path.clone();
                    let mod_name = module.name(db).value(db);
                    module_dir_path.push(mod_name);
                    file_path.push(format!("{}.lopa", mod_name));
                    if let Some(file) = files_by_names.get(&file_path) {
                        tree.modules_by_files.insert(*file, module);
                        tree.files_by_modules.insert(module, *file);

                        let mut children = vec![];
                        for item in file.items(db).items(db).iter() {
                            if let hir::Item::Module(child) = item {
                                children.push(*child);
                                tree.parents.insert(*child, module);
                                traverse_module(
                                    db,
                                    *child,
                                    module_dir_path.clone(),
                                    tree,
                                    files_by_names,
                                );
                            }
                        }
                        tree.children.insert(module, children.into());
                    } else {
                        tree.diagnostics.push(Diagnostic {
                            message: format!("unresolved module: `{}`", mod_name),
                            location: DiagnosticLocation::Module(
                                module.id(db).expect("declaration modules must have ids"),
                            ),
                            kind: DiagnosticKind::ModuleError,
                        });
                    }
                }
                hir::ModuleKind::Definition => {
                    let mut children = vec![];
                    for child in module.modules(db).modules(db).iter() {
                        children.push(*child);
                        tree.parents.insert(*child, module);
                        traverse_module(db, *child, module_dir_path.clone(), tree, files_by_names);
                    }
                    tree.children.insert(module, children.into());
                }
                hir::ModuleKind::Root => {
                    let root_file = module.root(db).root_file(db).unwrap();
                    tree.modules_by_files.insert(root_file, module);
                    tree.files_by_modules.insert(module, root_file);
                    let mut children = vec![];
                    for child in module.modules(db).modules(db).iter() {
                        children.push(*child);
                        tree.parents.insert(*child, module);
                        traverse_module(db, *child, module_dir_path.clone(), tree, files_by_names);
                    }
                    tree.children.insert(module, children.into());
                }
            }
        }
        let mut tree = ModuleTree {
            parents: Default::default(),
            children: Default::default(),
            modules_by_files: Default::default(),
            files_by_modules: Default::default(),
            diagnostics: Default::default(),
        };
        let Some(root_module) = self.root_module(db) else {
            return Default::default();
        };
        let mut root_dir = self.root_dir(db).clone();
        root_dir.push("src");

        traverse_module(db, root_module, root_dir, &mut tree, files_by_names);

        tree
    }

    #[salsa::tracked(returns(clone))]
    pub fn root_file(self, db: &'db dyn salsa::Database) -> Option<File> {
        let mut root_file_path = self.root_dir(db).clone();
        root_file_path.push("src");
        root_file_path.push("main.lopa");
        for file in self.files(db) {
            if file.path(db) == &root_file_path {
                return Some(*file);
            }
        }
        None
    }

    #[salsa::tracked(returns(clone))]
    pub fn root_module(self, db: &'db dyn salsa::Database) -> Option<hir::Module<'db>> {
        let root_file = self.root_file(db)?;
        let items = root_file.items(db);
        Some(hir::Module::new(
            db,
            Symbol::new(db, "root"),
            hir::ModuleData::Root { items },
            hir::ModuleKind::Root,
            root_file.root(db),
        ))
    }

    #[salsa::tracked(returns(ref))]
    pub fn files_by_names(self, db: &'db dyn salsa::Database) -> indexmap::IndexMap<PathBuf, File> {
        self.files(db)
            .iter()
            .map(|file| (file.path(db).clone(), *file))
            .collect::<_>()
    }
}

#[salsa::input]
pub struct File {
    #[returns(ref)]
    pub contents: Arc<str>,
    pub path: PathBuf,
    #[returns(clone)]
    pub root: Root,
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("File").field(&self.0).finish()
    }
}

#[derive(Clone, salsa::SalsaValue, PartialEq, Eq, Hash, Debug)]
pub struct InFile<T> {
    pub value: T,
    pub file: File,
}

impl<T: Copy> Copy for InFile<T> {}

impl<T> InFile<T> {
    pub fn new(value: T, file: File) -> Self {
        Self { value, file }
    }
}

#[salsa::tracked]
impl<'db> hir::Module<'db> {
    #[salsa::tracked(returns(copy))]
    pub fn modules(self, db: &'db dyn salsa::Database) -> hir::Modules<'db> {
        hir::Modules::new(
            db,
            self.items(db)
                .items(db)
                .iter()
                .filter_map(|item| {
                    if let hir::Item::Module(item) = item {
                        Some(*item)
                    } else {
                        None
                    }
                })
                .collect_vec(),
        )
    }
    #[salsa::tracked(returns(copy))]
    pub fn items(self, db: &'db dyn salsa::Database) -> hir::Items<'db> {
        match self.data(db) {
            hir::ModuleData::Root { items } => items,
            hir::ModuleData::Definition { items, .. } => items,
            hir::ModuleData::Declaration { .. } => self
                .file(db)
                .map(|f| f.items(db))
                .unwrap_or_else(|| hir::Items::new(db, vec![])),
        }
    }

    pub fn file(self, db: &'db dyn salsa::Database) -> Option<File> {
        match self.data(db) {
            hir::ModuleData::Root { .. } => self.root(db).root_file(db),
            hir::ModuleData::Definition { .. } => None,
            hir::ModuleData::Declaration { .. } => {
                let module_tree = self.root(db).module_tree(db);
                module_tree.files_by_modules.get(&self).cloned()
            }
        }
    }
    //TODO: remove all unwraps

    #[salsa::tracked(returns(copy))]
    pub fn absolute_path(self, db: &'db dyn salsa::Database) -> def::SymbolList {
        if self.is_root_module(db) {
            return SymbolList::new(db, [Symbol::new(db, "root")]);
        };
        let mut path = vec![self.name(db)];
        let module_tree = self.root(db).module_tree(db);

        let mut current = self;
        while let Some(parent) = module_tree.parents.get(&current) {
            path.insert(0, parent.name(db));
            current = *parent;
        }

        SymbolList::new(db, path)
    }

    #[salsa::tracked(returns(copy))]
    fn is_root_module(self, db: &'db dyn salsa::Database) -> bool {
        self.file(db) == self.root(db).root_file(db)
    }

    #[salsa::tracked(returns(copy))]
    pub fn parent(self, db: &'db dyn salsa::Database) -> Option<hir::Module<'db>> {
        let module_tree = self.root(db).module_tree(db);
        module_tree.parents.get(&self).cloned()
    }

    #[salsa::tracked(returns(clone))]
    pub fn children(self, db: &'db dyn salsa::Database) -> Arc<Vec<hir::Module<'db>>> {
        let module_tree = self.root(db).module_tree(db);
        module_tree
            .children
            .get(&self)
            .cloned()
            .unwrap_or_else(|| Arc::new(vec![]))
    }
}

#[salsa::tracked(returns(clone))]
pub fn module_diagnostics<'db>(
    db: &'db dyn salsa::Database,
    module: hir::Module<'db>,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    diagnostics.extend(resolve_module(db, module));
    diagnostics.extend(&mut module_scope::accumulated(db, module).into_iter().cloned());

    for child in module.children(db).iter() {
        if matches!(child.data(db), hir::ModuleData::Declaration { .. }) {
            continue;
        }
        let mut child_diagnostics = module_diagnostics(db, *child);
        diagnostics.append(&mut child_diagnostics);
    }
    diagnostics
}

#[salsa::tracked]
impl<'db> File {
    #[salsa::tracked(returns(copy))]
    pub fn parse(self, db: &'db dyn salsa::Database) -> parsing::Parse<'db> {
        let (tree, errors) = parsing::parse(self.contents(db));
        parsing::Parse::new(db, tree, errors)
    }

    //So, here's an issue. Currently diagnostics rely on accumulated values. That's a problem,
    //because if some query produces a diagnostic and ends up being nested into other queries (as is
    //the case with `module_tree`), diagnostics become duplicated.
    //Solution 1: abandon accumulators in favor of manual diagnostic collection.
    //Solution 2: dedup them at the end.

    #[salsa::tracked(returns(clone))]
    pub fn diagnostics(self, db: &dyn salsa::Database) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let parse = self.parse(db);
        let module_tree = self.root(db).module_tree(db);
        diagnostics.extend(parse.errors(db).clone().into_iter().map(|e| Diagnostic {
            message: e.kind.to_string(),
            location: DiagnosticLocation::Range(e.range.clone()),
            kind: DiagnosticKind::SyntaxError,
        }));

        diagnostics.extend(module_tree.diagnostics.clone());

        if let Some(module) = self.module(db) {
            diagnostics.extend(module_diagnostics(db, module));
        }
        diagnostics
    }
}

#[salsa::tracked]
impl<'db> File {
    #[salsa::tracked(returns(copy))]
    pub fn module(self, db: &'db dyn salsa::Database) -> Option<hir::Module<'db>> {
        let module_tree = self.root(db).module_tree(db);
        module_tree.modules_by_files.get(&self).cloned()
    }

    #[salsa::tracked(returns(clone))]
    pub fn rendered_diagnostics(self, db: &'db dyn salsa::Database) -> Vec<RenderedDiagnostic> {
        let parse = self.parse(db);
        let tree = parse.tree(db);
        let diagnostics = self.diagnostics(db);
        diagnostics
            .into_iter()
            .filter_map(|d| match d.location {
                DiagnosticLocation::TypeExpr { id, source } => {
                    let id = match source {
                        hir::IdSourcePure::BodySource(body_map) => body_map[id],
                        hir::IdSourcePure::ContentsSource(contents_map) => contents_map[id],
                    };
                    let node = tree.get(id)?;
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range: node.range(),
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::Struct(ast_id) => {
                    let id = ast_id.file.ast_map(db)[ast_id];
                    let node = tree.get(id).and_then(parsing::StructItem::cast)?;
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range: {
                            if let Some(struct_token) = node.struct_token()
                                && let Some(name) = node.name()
                            {
                                struct_token.range().start..name.syntax().range().end
                            } else {
                                node.syntax().range()
                            }
                        },
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::Enum(ast_id) => {
                    let id = ast_id.file.ast_map(db)[ast_id];
                    let node = tree.get(id).and_then(parsing::EnumItem::cast)?;
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range: {
                            if let Some(struct_token) = node.enum_token()
                                && let Some(name) = node.name()
                            {
                                struct_token.range().start..name.syntax().range().end
                            } else {
                                node.syntax().range()
                            }
                        },
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::Function(ast_id) => {
                    let id = ast_id.file.ast_map(db)[ast_id];
                    let node = tree.get(id).and_then(parsing::FnItem::cast)?;
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range: {
                            if let Some(struct_token) = node.fn_token()
                                && let Some(name) = node.name()
                            {
                                struct_token.range().start..name.syntax().range().end
                            } else {
                                node.syntax().range()
                            }
                        },
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::Module(ast_id) => {
                    let id = ast_id.file.ast_map(db)[ast_id];
                    let node = tree.get(id).and_then(parsing::ModItem::cast)?;
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range: node.syntax().range(),
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::UseTree { use_id, tree_id } => {
                    let use_item = use_id.file.items_map(db)[use_id];
                    let use_tree_map = use_item.use_tree_map(db)?;
                    let use_tree = use_tree_map[tree_id];
                    let node = tree.get(use_tree)?;
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range: node.range(),
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::Range(range) => Some(RenderedDiagnostic {
                    message: d.message,
                    range,
                    kind: d.kind,
                }),
                DiagnosticLocation::Param { fn_item, param_num } => None,
            })
            .collect_vec()
    }
}
