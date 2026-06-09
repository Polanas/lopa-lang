pub mod base;
pub mod diagnostics;

use notify_rust::Notification;
use rowan::GreenNode;
use rowan::ast::AstNode as _;
use salsa::{Accumulator, Database, Setter};
use ustr::Ustr;

use crate::def::ir::{Function, ImplItem, Struct, Type};
use crate::def::lower::{self, impl_blocks};
use crate::def::{self, ir};
use crate::ide::base::VfsPath;
use crate::ide::diagnostics::Diagnostic;
use crate::parsing::ast::{self, SyntaxNode};
use crate::parsing::parser::{self};
use crate::ty::infer;
use crate::ustr_hash::UstrIndexMap;
use std::fmt::Display;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct FileContent {
    value: String,
    line_starts: Vec<usize>,
    dirty: bool,
}

impl Display for FileContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn line_starts(contents: &str) -> Vec<usize> {
    let mut it = contents.bytes();
    let mut line_starts = vec![0];
    let mut count = 0;
    while let Some(byte) = it.next() {
        match byte {
            b'\n' | b'\r' => {
                if byte == b'\r' {
                    //skip \n
                    it.next();
                    count += 1;
                }
                line_starts.push(count + 1);
            }
            _ => {}
        }
        count += 1;
    }
    line_starts
}

impl FileContent {
    pub fn new(contents: String) -> Self {
        Self {
            line_starts: line_starts(contents.as_str()),
            value: contents,
            dirty: false,
        }
    }

    fn try_recompute(&mut self) {
        if self.dirty {
            self.line_starts = line_starts(self.value.as_str());
            self.dirty = false;
        }
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn len(&self) -> usize {
        self.value.as_str().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, offset: usize, text: &str) {
        self.value.insert_str(offset, text);
        self.dirty = true;
    }

    pub fn delete(&mut self, range: Range<usize>) {
        self.value.replace_range(range, "");
        self.dirty = true;
    }

    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        self.value.replace_range(range, text);
        self.dirty = true;
    }

    pub fn line_col_by_pos(&mut self, pos: u32) -> (u32, u32) {
        let pos = pos as usize;
        self.try_recompute();
        let line = self
            .line_starts
            .partition_point(|&i| i <= pos)
            .saturating_sub(1);
        let col = pos - self.line_starts[line];
        (line as _, col as _)
    }

    pub fn pos_by_line_col(&mut self, line: u32, col: u32) -> u32 {
        let (line, col) = (line as usize, col as usize);
        self.try_recompute();
        let pos = self.line_starts.get(line).copied().unwrap_or(0);
        (pos + col) as u32
    }
}

#[salsa::input]
#[derive(Debug)]
pub struct File {
    #[returns(ref)]
    pub contents: Arc<RwLock<FileContent>>,
    #[returns(ref)]
    pub path: VfsPath,
    pub source_root: SourceRoot,
}

#[salsa::tracked]
pub fn is_root_file(db: &dyn salsa::Database, file: File) -> bool {
    module_name(db, file) == "root"
}

#[salsa::tracked]
pub fn module_path(db: &dyn salsa::Database, file: File) -> ir::Path {
    let mut path = ir::Path(vec![module_name(db, file)]);
    let mut current = file;
    while let Some(parent) = lower::module_parent(db, current) {
        path.0.insert(0, module_name(db, parent));
        current = parent;
    }
    path
}

#[salsa::tracked]
pub fn module_name(db: &dyn salsa::Database, file: File) -> Ustr {
    let name = file
        .path(db)
        .0
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string()
        .into();
    if name == "main" {
        Ustr::from("root")
    } else {
        name
    }
}

#[salsa::input]
#[derive(Debug)]
pub struct SourceRoot {
    pub files: Option<Vec<File>>,
    pub path: PathBuf,
}

#[salsa::tracked]
pub fn root_module(db: &dyn salsa::Database, root: SourceRoot) -> Option<File> {
    root.files(db)?
        .iter()
        .find(|file| is_root_file(db, **file))
        .cloned()
}

impl SourceRoot {
    pub fn clear(&self, db: &mut dyn salsa::Database) {
        self.set_files(db).to(Some(vec![]));
    }

    pub fn push_file(&self, db: &mut dyn salsa::Database, file: File) {
        let Some(mut files) = self.set_files(db).to(None) else {
            return;
        };
        files.push(file);
        self.set_files(db).to(Some(files));
    }
}

#[salsa::tracked]
pub struct Parse<'db> {
    pub node: GreenNode,
    #[returns(ref)]
    pub errors: Vec<parser::ParseError>,
}

impl Parse<'_> {
    pub fn syntax_node(&self, db: &dyn salsa::Database) -> ast::SyntaxNode {
        SyntaxNode::new_root(self.node(db).clone())
    }

    pub fn file(&self, db: &dyn salsa::Database) -> ast::File {
        ast::File::cast(self.syntax_node(db)).unwrap()
    }
}

#[salsa::tracked]
pub fn parse<'db>(db: &'db dyn salsa::Database, file: File) -> Parse<'db> {
    let contents = file.contents(db).read().unwrap();
    let parse = parser::parse(contents.as_str());

    Parse::new(db, parse.0, parse.1)
}

#[salsa::tracked]
pub fn body_with_source_map<'db>(
    db: &'db dyn salsa::Database,
    func: Function<'db>,
) -> (
    Arc<def::body::Body<'db>>,
    Arc<def::body::BodySourceMap<'db>>,
) {
    let (body, body_source_map) = def::body::lower(db, func);
    (Arc::new(body), Arc::new(body_source_map))
}

#[salsa::tracked(returns(ref))]
pub fn body<'db>(db: &'db dyn salsa::Database, func: Function<'db>) -> Arc<def::body::Body<'db>> {
    body_with_source_map(db, func).0.clone()
}

#[salsa::tracked(returns(ref))]
pub fn source_map<'db>(
    db: &'db dyn salsa::Database,
    func: Function<'db>,
) -> Arc<def::body::BodySourceMap<'db>> {
    body_with_source_map(db, func).1.clone()
}

#[derive(salsa::Update, Debug, PartialEq, Eq, Clone, Hash)]
pub enum Implementee<'db> {
    Type(ir::Type<'db>),
    Itself,
}

#[derive(salsa::Update, Debug, PartialEq, Eq, Clone, Hash)]
pub struct ImplKey<'db> {
    //the type that items are implemented ON
    pub implementor: ir::Type<'db>,
    //the type that items are implemented FROM (acts as an interface)
    pub implementee: Implementee<'db>,
}

type ImplMap<'db> = indexmap::IndexMap<ImplKey<'db>, UstrIndexMap<ImplItem<'db>>>;

#[salsa::tracked(returns(ref))]
pub fn impl_map(db: &dyn salsa::Database, root: SourceRoot) -> ImplMap<'_> {
    let mut impls: ImplMap = Default::default();
    for &file in root.files(db).unwrap().iter() {
        let lower = impl_blocks(db, file);
        for block in lower.impl_blocks(db) {
            let implementor = block.owner(db);
            let implementee = block
                .implementee(db)
                .map(Implementee::Type)
                .unwrap_or_else(|| Implementee::Itself);
            let items = impls
                .entry(ImplKey {
                    implementor,
                    implementee,
                })
                .or_default();
            for func in block.functions(db) {
                items.insert(func.name(db).into(), ImplItem::Function(*func));
            }
        }
    }

    impls
}

#[salsa::db]
#[derive(Default)]
pub struct RootDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for RootDatabase {}

pub struct Analysis {
    pub db: RootDatabase,
}

impl Analysis {
    pub fn new() -> Self {
        let db = RootDatabase::default();
        Self { db }
    }

    pub fn diagnostics(&self, file: File) -> Vec<Diagnostic> {
        diagnostics::file_diagnostics(&self.db, file)
    }

    pub fn apply_change(&mut self, file: File) {
        //cancel any ongoing analysis
        self.db.trigger_cancellation();

        let contents = file.contents(&self.db).clone();
        file.set_contents(&mut self.db).to(contents);
    }
}

impl Default for Analysis {
    fn default() -> Self {
        Self::new()
    }
}

pub type TextRange = rowan::TextRange;
