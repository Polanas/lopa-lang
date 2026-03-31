use crop::Rope;
use lopa_core::parsing::{self, parser};
use tower_lsp_server::ls_types::*;

#[salsa::input]
pub struct SourceUri {
    pub value: Uri,
}


#[salsa::tracked(debug)]
pub struct ParseResult<'db> {
    #[tracked]
    #[returns(ref)]
    pub parse: parser::Parse,
}

// pub fn source(db: &dyn salsa::Database, uri: SourceUri) -> Option<SourceFile<'_>> {
//     let path = Url::parse(uri.value(db).as_str()).ok()?;
//     Some(SourceFile::new(
//         db,
//         Rope::from(std::fs::read_to_string(path.as_str()).ok()?).into(),
//     ))
// }

// #[salsa::tracked]
// pub fn parse(db: &dyn salsa::Database, source: SourceFile) -> ParseResult<'_> {
//     ParseResult::new(db, parser::parse(source.text(db).as_ref()))
// }

#[salsa::db]
#[derive(Default)]
pub struct DbImpl {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for DbImpl {}
