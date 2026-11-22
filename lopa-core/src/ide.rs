mod def_map;
mod diagnostics;
mod lower;
mod resolve;
mod scope;
mod type_check;

pub use diagnostics::{
    Diagnostic, DiagnosticKind, DiagnosticLocation, RenderedDiagnostic, Severity,
};

pub use def_map::*;
use la_arena::{Arena, Idx};
pub use resolve::*;
pub use scope::*;

use itertools::Itertools;

use crate::{
    common::PathNodeType,
    def::{
        self, Symbol, SymbolList,
        hir::{self, Item},
    },
    parsing::{self, AstNode},
};
use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};

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

#[derive(Clone, Default, PartialEq, salsa::SalsaValue, Debug)]
pub struct ModuleTree<'db> {
    pub parents: indexmap::IndexMap<hir::Module<'db>, hir::Module<'db>>,
    pub children: indexmap::IndexMap<hir::Module<'db>, hir::Modules<'db>>,
    pub modules_by_paths: indexmap::IndexMap<PathBuf, hir::Module<'db>>,
    pub paths_by_modules: indexmap::IndexMap<hir::Module<'db>, PathBuf>,
}

impl<'db> ModuleTree<'db> {
    fn insert_module_path(&mut self, path: &std::path::Path, module: hir::Module<'db>) {
        self.modules_by_paths.insert(path.to_path_buf(), module);
        self.paths_by_modules.insert(module, path.to_path_buf());
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DirNodeKind {
    File(File),
    Dir(Vec<Idx<DirNode>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DirNode {
    path: PathBuf,
    kind: DirNodeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DirTree {
    nodes: Arena<DirNode>,
    root: Idx<DirNode>,
}

struct DirTreeCtx<'a> {
    files: &'a indexmap::IndexMap<PathBuf, File>,
    root_path: &'a Path,
}

impl DirTree {
    fn new<'a>(ctx: DirTreeCtx<'a>) -> Self {
        let mut nodes = Arena::new();

        let root = nodes.alloc(DirNode {
            path: PathBuf::new(),
            kind: DirNodeKind::Dir(vec![]),
        });

        let mut directories = HashMap::new();
        directories.insert(PathBuf::new(), root);

        for (path, file) in ctx.files.iter() {
            let mut parent = root;
            let mut current_path = PathBuf::new();

            let parent_path = path.strip_prefix(ctx.root_path).unwrap().parent().unwrap();
            for component in parent_path.components() {
                current_path.push(component);
                let dir = if let Some(dir) = directories.get(&current_path) {
                    *dir
                } else {
                    let dir = nodes.alloc(DirNode {
                        path: current_path.clone(),
                        kind: DirNodeKind::Dir(vec![]),
                    });

                    if let DirNodeKind::Dir(children) = &mut nodes[parent].kind {
                        children.push(dir);
                    }
                    directories.insert(current_path.clone(), dir);

                    dir
                };

                parent = dir;
            }

            let file = nodes.alloc(DirNode {
                path: path.clone(),
                kind: DirNodeKind::File(*file),
            });
            if let DirNodeKind::Dir(children) = &mut nodes[parent].kind {
                children.push(file);
            }
        }

        Self { nodes, root }
    }

    fn stringify(&self) -> String {
        fn stringify(tree: &DirTree, node: Idx<DirNode>, output: &mut String, depth: usize) {
            (0..depth).for_each(|_| output.push_str("  "));
            match &tree.nodes[node].kind {
                DirNodeKind::File(_) => output.push_str(&format!(
                    "{},\n",
                    tree.nodes[node].path.file_stem().unwrap().to_string_lossy()
                )),
                DirNodeKind::Dir(items) => {
                    output.push_str(&format!(
                        "{}:\n",
                        tree.nodes[node]
                            .path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| String::from("root"))
                    ));
                    for item in items.iter() {
                        stringify(tree, *item, output, depth + 1);
                    }
                    (0..depth).for_each(|_| output.push_str("  "));
                }
            }
        }
        let mut output = String::new();
        stringify(self, self.root, &mut output, 0);
        output
    }
}

struct ScanCtx<'db, 'a> {
    db: &'db dyn salsa::Database,
    tree: &'a mut ModuleTree<'db>,
    files: &'a indexmap::IndexMap<PathBuf, File>,
    dir_tree: &'a DirTree,
    root: Root,
}

impl<'db, 'a> ScanCtx<'db, 'a> {
    fn process_module(&mut self, node: Idx<DirNode>) -> hir::Module<'db> {
        let node = &self.dir_tree.nodes[node];
        let name = node.path.file_stem().unwrap().to_string_lossy().to_string();
        match &node.kind {
            DirNodeKind::Dir(items) => {
                let origin = hir::ModuleOrigin::Directory {
                    path: node.path.clone(),
                    file: {
                        let path = node.path.join(&name);
                        if path.exists() {
                            Some(*self.files.get(&path).unwrap())
                        } else {
                            None
                        }
                    },
                };
                let file_path = if let hir::ModuleOrigin::Directory {
                    file: Some(file), ..
                } = origin
                {
                    Some(file.path(self.db))
                } else {
                    None
                };
                let module =
                    hir::Module::new(self.db, Symbol::new(self.db, name), self.root, origin);
                self.tree.insert_module_path(&node.path, module);
                if let Some(path) = file_path {
                    self.tree.modules_by_paths.insert(path.clone(), module);
                }
                let mut children = vec![];
                for entry in items.iter() {
                    let child = self.process_module(*entry);
                    self.tree.parents.insert(child, module);
                    children.push(module);
                }
                self.tree
                    .children
                    .insert(module, hir::Modules::new(self.db, children));
                module
            }
            DirNodeKind::File(file) => {
                if let (Some(root_file), Some(root_module)) =
                    (self.root.root_file(self.db), self.root.root_module(self.db))
                    && *file == root_file
                {
                    return root_module;
                }
                let module = hir::Module::new(
                    self.db,
                    Symbol::new(self.db, name),
                    self.root,
                    hir::ModuleOrigin::File(*file),
                );
                self.tree.insert_module_path(&node.path, module);
                module
            }
        }
    }
}

#[salsa::tracked]
impl<'db> Root {
    #[salsa::tracked(returns(ref))]
    pub fn module_tree(self, db: &'db dyn salsa::Database) -> ModuleTree<'db> {
        let files = self.files_by_paths(db);
        let mut tree = ModuleTree {
            parents: Default::default(),
            children: Default::default(),
            modules_by_paths: Default::default(),
            paths_by_modules: Default::default(),
        };
        let (Some(root_module), Some(root_file)) = (self.root_module(db), self.root_file(db))
        else {
            return Default::default();
        };
        tree.insert_module_path(root_file.path(db), root_module);

        let mut root_path = self.root_dir(db).clone();
        root_path.push("src");

        let dir_tree = DirTree::new(DirTreeCtx {
            files,
            root_path: &root_path,
        });

        let mut ctx = ScanCtx {
            db,
            tree: &mut tree,
            files,
            root: self,
            dir_tree: &dir_tree,
        };

        let DirNodeKind::Dir(nodes) = &ctx.dir_tree.nodes[ctx.dir_tree.root].kind else {
            unreachable!()
        };

        let mut children = vec![];
        for entry in nodes.iter() {
            let module = ctx.process_module(*entry);
            if !module.is_root_module(db) {
                children.push(module);
            }
        }

        ctx.tree
            .children
            .insert(root_module, hir::Modules::new(db, children));

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
        Some(hir::Module::new(
            db,
            Symbol::new(db, "root"),
            root_file.root(db),
            hir::ModuleOrigin::Root(root_file),
        ))
    }

    #[salsa::tracked(returns(ref))]
    pub fn files_by_paths(self, db: &'db dyn salsa::Database) -> indexmap::IndexMap<PathBuf, File> {
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
    //Only explicitly defined items
    #[salsa::tracked(returns(copy))]
    pub fn inner_items(self, db: &'db dyn salsa::Database) -> hir::Items<'db> {
        match self.origin(db) {
            hir::ModuleOrigin::Root(file) | hir::ModuleOrigin::File(file) => file.items(db),
            hir::ModuleOrigin::Directory { file, .. } => file
                .map(|f| f.items(db))
                .unwrap_or_else(|| hir::Items::new(db, vec![])),
            hir::ModuleOrigin::Definition { .. } => self.items(db),
        }
    }

    //All items of a module, including ones not explicitly defined (relevant for root and dir modules)
    #[salsa::tracked(returns(copy))]
    pub fn items(self, db: &'db dyn salsa::Database) -> hir::Items<'db> {
        match self.origin(db) {
            hir::ModuleOrigin::File(file) => file.items(db),
            hir::ModuleOrigin::Directory { file, .. } => {
                let mut child_items = self
                    .root(db)
                    .module_tree(db)
                    .children
                    .get(&self)
                    .unwrap()
                    .modules(db)
                    .iter()
                    .map(|m| hir::Item::Module(*m))
                    .collect_vec();

                if let &Some(file) = file {
                    for &item in file.items(db).items(db) {
                        child_items.push(item);
                    }
                }
                hir::Items::new(db, child_items)
            }
            hir::ModuleOrigin::Definition { file, .. } => {
                *file.lower(db).items_by_modules.get(&self).unwrap()
            }
            hir::ModuleOrigin::Root(file) => {
                let mut child_items = self
                    .root(db)
                    .module_tree(db)
                    .children
                    .get(&self)
                    .unwrap()
                    .modules(db)
                    .iter()
                    .map(|m| hir::Item::Module(*m))
                    .collect_vec();

                for &item in file.items(db).items(db) {
                    child_items.push(item);
                }
                hir::Items::new(db, child_items)
            }
        }
    }

    pub fn file(self, db: &'db dyn salsa::Database) -> Option<File> {
        Some(match self.origin(db) {
            hir::ModuleOrigin::File(file) | hir::ModuleOrigin::Root(file) => *file,
            hir::ModuleOrigin::Directory { file, .. } => return *file,
            hir::ModuleOrigin::Definition { .. } => return None,
        })
    }

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
        match self.origin(db) {
            hir::ModuleOrigin::Root(_) => None,
            hir::ModuleOrigin::File(_) | hir::ModuleOrigin::Directory { .. } => {
                module_tree.parents.get(&self).cloned()
            }
            hir::ModuleOrigin::Definition { parent, .. } => Some(*parent),
        }
    }

    #[salsa::tracked(returns(clone))]
    pub fn children(self, db: &'db dyn salsa::Database) -> hir::Modules<'db> {
        hir::Modules::new(
            db,
            self.items(db)
                .items(db)
                .iter()
                .filter_map(|i| {
                    if let Item::Module(m) = i {
                        Some(*m)
                    } else {
                        None
                    }
                })
                .collect_vec(),
        )
    }

    #[salsa::tracked(returns(clone))]
    pub fn diagnostics(self, db: &'db dyn salsa::Database) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        diagnostics.extend(self.resolved_scope(db).diagnostics.clone());

        for item in self.inner_items(db).items(db) {
            match item {
                hir::Item::Function(item) => {
                    // let path_map = item.path_map(db, self);
                    // diagnostics.extend(path_map.diagnostics.clone());
                }
                hir::Item::Struct(_) => {}
                hir::Item::Enum(_) => {}
                hir::Item::Use(item) => {
                    // let path_map = item.path_map(db, self);
                    // diagnostics.extend(path_map.diagnostics.clone());
                }
                hir::Item::Impl(impl_block) => {}
                hir::Item::Module(item) => {
                    diagnostics.extend(item.diagnostics(db).clone());
                }
                hir::Item::Class(class) => {}
                hir::Item::Trait(_) => {}
            }
        }
        diagnostics
    }
}

#[salsa::tracked]
impl<'db> File {
    #[salsa::tracked(returns(copy))]
    pub fn parse(self, db: &'db dyn salsa::Database) -> parsing::Parse<'db> {
        let (tree, errors) = parsing::parse(self.contents(db));
        parsing::Parse::new(db, tree, errors)
    }

    //Diagnostics collection should AVOID accumulators, and instead collect and return them
    //manually, for reasons
    #[salsa::tracked(returns(clone))]
    pub fn diagnostics(self, db: &dyn salsa::Database) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let parse = self.parse(db);
        diagnostics.extend(parse.errors(db).clone().into_iter().map(|e| Diagnostic {
            message: e.kind.to_string(),
            location: DiagnosticLocation::Range(e.range.clone()),
            kind: DiagnosticKind::SyntaxError,
        }));

        if let Some(module) = self.module(db) {
            diagnostics.extend(module.diagnostics(db));
        }
        diagnostics
    }
}

#[salsa::tracked]
impl<'db> File {
    #[salsa::tracked(returns(copy))]
    pub fn module(self, db: &'db dyn salsa::Database) -> Option<hir::Module<'db>> {
        let module_tree = self.root(db).module_tree(db);
        module_tree.modules_by_paths.get(self.path(db)).cloned()
    }

    #[salsa::tracked(returns(clone))]
    pub fn rendered_diagnostics(self, db: &'db dyn salsa::Database) -> Vec<RenderedDiagnostic> {
        let parse = self.parse(db);
        let tree = parse.tree(db);
        let diagnostics = self.diagnostics(db);
        diagnostics
            .into_iter()
            .filter_map(|d| match d.location {
                DiagnosticLocation::PathSegment {
                    id,
                    source,
                    offset,
                    node_type,
                } => {
                    let id = source[id];
                    let node = tree.get(id)?;
                    let path = match node_type {
                        PathNodeType::Expr => {
                            let path_expr = parsing::PathExpr::cast(node).unwrap();
                            path_expr.path()
                        }
                        PathNodeType::Record => {
                            let record = parsing::RecordExpr::cast(node).unwrap();
                            record.path()
                        }
                        PathNodeType::Type => {
                            let path_type = parsing::PathType::cast(node).unwrap();
                            path_type.path()
                        }
                        PathNodeType::Pat => {
                            let path_pat = parsing::PathPattern::cast(node).unwrap();
                            path_pat.path()
                        }
                    }
                    .unwrap();

                    //TODO: apparently this can crash(?)
                    let segment = path.segments().nth(offset).unwrap();
                    let range = segment.0.children().next().unwrap().range();

                    Some(RenderedDiagnostic {
                        message: d.message,
                        range,
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::HirNode { id, source } => {
                    let id = source[id];
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
                            if let Some(token) = node.struct_token()
                                && let Some(name) = node.name()
                            {
                                token.range().start..name.syntax().range().end
                            } else {
                                node.syntax().range()
                            }
                        },
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::Class(ast_id) => {
                    let id = ast_id.file.ast_map(db)[ast_id];
                    let node = tree.get(id).and_then(parsing::ClassItem::cast)?;
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range: {
                            if let Some(token) = node.class_token()
                                && let Some(name) = node.name()
                            {
                                token.range().start..name.syntax().range().end
                            } else {
                                node.syntax().range()
                            }
                        },
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::Trait(ast_id) => {
                    let id = ast_id.file.ast_map(db)[ast_id];
                    let node = tree.get(id).and_then(parsing::TraitItem::cast)?;
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range: {
                            if let Some(token) = node.trait_token()
                                && let Some(name) = node.name()
                            {
                                token.range().start..name.syntax().range().end
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
                            if let Some(token) = node.enum_token()
                                && let Some(name) = node.name()
                            {
                                token.range().start..name.syntax().range().end
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
                    let use_item = use_id.file.items_map(db)[use_id].item;
                    let use_tree_map = use_item.use_tree_map(db)?;
                    let use_tree = use_tree_map[tree_id];
                    let node = tree.get(use_tree)?;
                    let range = node.children().next().unwrap().range();
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range,
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

mod test {
    use std::{path::PathBuf, sync::Arc};

    use salsa::DatabaseImpl;

    use crate::ide::{DirTree, DirTreeCtx, File, Root};

    #[test]
    fn dir_tree() {
        let db = DatabaseImpl::new();
        let root_dir = PathBuf::from("project/src");
        let root = Root::new(&db, vec![], root_dir.clone());
        let files = vec![
            File::new(
                &db,
                Arc::from(""),
                PathBuf::from("project/src/main.lopa"),
                root,
            ),
            File::new(
                &db,
                Arc::from(""),
                PathBuf::from("project/src/game/player.lopa"),
                root,
            ),
            File::new(
                &db,
                Arc::from(""),
                PathBuf::from("project/src/game/enemy.lopa"),
                root,
            ),
        ];

        let files: indexmap::IndexMap<_, _> = files
            .into_iter()
            .map(|f| (f.path(&db).to_path_buf(), f))
            .collect();
        let tree = DirTree::new(DirTreeCtx {
            files: &files,
            root_path: &root_dir,
        });
        insta::assert_snapshot!(&tree.stringify());
    }
}
