use crate::ide;

#[salsa::tracked]
pub fn format_file<'db>(db: &'db dyn salsa::Database, file: ide::File) -> String {
    String::from("formatted")
}
