pub mod base;
pub mod diagnostics;

use notify_rust::Notification;
use rowan::GreenNode;
use rowan::ast::AstNode as _;
use salsa::{Accumulator, Database, Setter};
use ustr::Ustr;

use crate::def::ir::{Function, ImplFunction, Struct, Type};
use crate::def::lower::{self, impl_blocks};
use crate::def::{self, ir};
use crate::ide::base::VfsPath;
use crate::ide::diagnostics::Diagnostic;
use crate::parsing::ast::{self, SyntaxNode};
use crate::parsing::parser::{self};
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
    module_name(db, file) == "main"
        && file
            .path(db)
            .0
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p == file.source_root(db).path(db).as_path())
            .unwrap_or(false)
}

#[salsa::tracked]
pub fn module_name(db: &dyn salsa::Database, file: File) -> Ustr {
    file.path(db)
        .0
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string()
        .into()
}

#[salsa::input]
#[derive(Debug)]
pub struct SourceRoot {
    pub files: Option<Vec<File>>,
    pub path: PathBuf,
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

#[derive(salsa::Update, Debug, PartialEq, Eq, Clone, Default)]
pub struct ImplFunctions<'db> {
    pub fns: Vec<ImplFunction<'db>>,
    pub from_impls: Vec<Type<'db>>,
}

// #[salsa::tracked(returns(ref))]
// pub fn impl_fns_list<'db>(
//     db: &'db dyn salsa::Database,
//     ty: Type<'db>,
//     files: Files,
// ) -> Option<Vec<ImplFunction<'db>>> {
//     let impl_fns = impls(db, files).functions.get(&ty)?;
//
//     let mut result = impl_fns.fns.clone();
//     for fns in impl_fns
//         .from_impls
//         .iter()
//         .filter(|t| **t != ty)
//         .filter_map(|t| impl_fns_list(db, t.clone(), files).clone())
//     {
//         result.extend(fns);
//     }
//     Some(result)
// }

type ImplMap<'db> = indexmap::IndexMap<ir::Type<'db>, ImplFunctions<'db>>;

#[derive(salsa::Update, Debug, Default, PartialEq, Eq, Clone)]
pub struct ImplItems<'db> {
    pub functions: ImplMap<'db>,
}

#[salsa::tracked(returns(ref))]
pub fn impls(db: &dyn salsa::Database, package: SourceRoot) -> ImplItems<'_> {
    let mut functions: ImplMap = Default::default();
    // for &file in files.value(db).as_ref().unwrap().iter() {
    //     let lower = impl_blocks(db, file);
    //
    //     for block in lower.impl_blocks(db) {
    //         let implementee = block.implementee(db);
    //         if let Some(impl_ty) = block.impl_ty(db)
    //             && impl_ty != block.implementee(db)
    //         {
    //             let fns = functions.entry(impl_ty.clone()).or_default();
    //             for func in block.methods(db) {
    //                 fns.fns.push(*func);
    //             }
    //             fns.from_impls.push(implementee);
    //         } else {
    //             let fns = functions.entry(implementee).or_default();
    //             for func in block.methods(db) {
    //                 fns.fns.push(*func);
    //             }
    //         }
    //     }
    // }

    ImplItems { functions }
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
        //TODO: use Cancelled::catch
        diagnostics::diagnostics(&self.db, file)
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
