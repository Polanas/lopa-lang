mod diagnostics;

pub use diagnostics::{
    Diagnostic, DiagnosticKind, DiagnosticLocation, RenderedDiagnostic, Severity,
};

use itertools::Itertools;
use salsa::Accumulator;

use crate::{
    def::{self, Symbol, hir},
    parsing::{self, AstNode},
};
use std::{path::PathBuf, sync::Arc};

#[salsa::input(debug)]
pub struct Root {
    #[returns(ref)]
    pub files: Vec<File>,
    #[returns(ref)]
    pub root_dir: PathBuf,
}

#[derive(Clone, Debug, PartialEq, salsa::Update)]
pub struct ModuleTree<'db> {
    pub parents: indexmap::IndexMap<hir::Module<'db>, hir::Module<'db>>,
    pub children: indexmap::IndexMap<hir::Module<'db>, Vec<hir::Module<'db>>>,
    pub modules_by_files: indexmap::IndexMap<File, hir::Module<'db>>,
}

#[salsa::tracked]
impl<'db> Root {
    #[salsa::tracked]
    pub fn root_file(self, db: &'db dyn salsa::Database) -> Option<File> {
        let mut root_file_path = self.root_dir(db).clone();
        root_file_path.push("src");
        root_file_path.push("main.lopa");
        for file in self.files(db) {
            if file.path(db) == root_file_path {
                return Some(*file);
            }
        }
        None
    }

    #[salsa::tracked]
    pub fn root_module(self, db: &'db dyn salsa::Database) -> Option<hir::Module<'db>> {
        let root_file = self.root_file(db)?;
        let items = root_file.items(db);
        Some(hir::Module::new(
            db,
            Symbol::new(db, "root"),
            hir::ModuleKind::Definition(items),
            root_file,
        ))
    }

    #[salsa::tracked(returns(ref))]
    pub fn files_by_names(self, db: &'db dyn salsa::Database) -> indexmap::IndexMap<PathBuf, File> {
        self.files(db)
            .iter()
            .map(|file| (file.path(db).clone(), *file))
            .collect::<_>()
    }
    pub fn module_tree(self, db: &'db dyn salsa::Database) -> &Option<ModuleTree<'db>> {
        module_tree(db, self)
    }
}

#[salsa::input(debug)]
pub struct File {
    #[returns(ref)]
    pub contents: Arc<str>,
    pub path: PathBuf,
    pub root: Root,
}

#[salsa::tracked(returns(ref))]
pub fn module_tree<'db>(db: &'db dyn salsa::Database, root: Root) -> Option<ModuleTree<'db>> {
    let files_by_names = root.files_by_names(db);
    fn traverse_module<'db>(
        db: &'db dyn salsa::Database,
        module: hir::Module<'db>,
        module_dir_path: PathBuf,
        tree: &mut ModuleTree<'db>,
        files_by_names: &indexmap::IndexMap<PathBuf, File>,
    ) {
        match module.kind(db) {
            hir::ModuleKind::Declaration(ast_ptr) => {
                let mut file_path = module_dir_path;
                let mod_name = module.name(db).value(db);
                file_path.push(format!("{}.lopa", mod_name));
                if let Some(file) = files_by_names.get(&file_path) {
                    tree.modules_by_files.insert(*file, module);
                } else {
                    Diagnostic {
                        message: format!("unresolved module: `{}`", mod_name),
                        location: DiagnosticLocation::Module(*ast_ptr),
                        kind: DiagnosticKind::ModuleError,
                    }
                    .accumulate(db);
                }
            }
            hir::ModuleKind::Definition(items) => {
                let mut children = vec![];
                for item in items.iter() {
                    if let hir::Item::Module(child) = item {
                        children.push(*child);
                        tree.parents.insert(*child, module);
                        traverse_module(db, *child, module_dir_path.clone(), tree, files_by_names);
                    }
                }
                tree.children.insert(module, children);
            }
        }
    }
    let mut tree = ModuleTree {
        parents: Default::default(),
        children: Default::default(),
        modules_by_files: Default::default(),
    };
    let root_module = root.root_module(db)?;
    let mut root_dir = root.root_dir(db).clone();
    root_dir.push("src");

    traverse_module(db, root_module, root_dir, &mut tree, files_by_names);

    Some(tree)
}

#[salsa::tracked]
pub fn module_diagnostics(db: &dyn salsa::Database, root: Root) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    diagnostics.extend(
        module_tree::accumulated::<Diagnostic>(db, root)
            .into_iter()
            .cloned(),
    );
    diagnostics
}

#[salsa::tracked]
impl<'db> File {
    #[salsa::tracked]
    pub fn parse(self, db: &'db dyn salsa::Database) -> parsing::Parse<'db> {
        let (tree, errors) = parsing::parse(self.contents(db));
        parsing::Parse::new(db, tree, errors)
    }

    #[salsa::tracked]
    pub fn diagnostics(self, db: &dyn salsa::Database) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let parse = self.parse(db);
        diagnostics.extend(parse.errors(db).clone().into_iter().map(|e| Diagnostic {
            message: e.kind.to_string(),
            location: DiagnosticLocation::Range(e.range.clone()),
            kind: DiagnosticKind::SyntaxError,
        }));
        diagnostics.extend(
            module_diagnostics::accumulated::<Diagnostic>(db, self.root(db))
                .into_iter()
                .cloned(),
        );
        diagnostics
    }
}

#[salsa::tracked]
impl<'db> File {
    #[salsa::tracked]
    pub fn rendered_diagnostics(self, db: &'db dyn salsa::Database) -> Vec<RenderedDiagnostic> {
        let parse = self.parse(db);
        let tree = parse.tree(db);
        let ast_map = self.ast_map(db);
        let diagnostics = self.diagnostics(db);
        diagnostics
            .into_iter()
            .filter_map(|d| match d.location {
                DiagnosticLocation::Module(ast_ptr) => {
                    let module_id = ast_map[ast_ptr];
                    let module_node = tree.get(module_id).and_then(parsing::ModItem::cast)?;
                    Some(RenderedDiagnostic {
                        message: d.message,
                        range: module_node.syntax().range(),
                        kind: d.kind,
                    })
                }
                DiagnosticLocation::Param { fn_item, param_num } => None,
                DiagnosticLocation::Range(range) => Some(RenderedDiagnostic {
                    message: d.message,
                    range,
                    kind: d.kind,
                }),
            })
            .collect_vec()
    }
}
