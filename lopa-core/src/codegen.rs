use crate::{def::ir, ide};

pub fn codegen_project<'db>(db: &'db dyn salsa::Database, source_root: ide::SourceRoot) -> String {
    todo!()
}

pub fn codegen_file<'db>(db: &'db dyn salsa::Database, file: ide::File) -> String {
    todo!()
}

pub fn codegen_function<'db>(db: &'db dyn salsa::Database, func: ir::Function<'db>) -> String {
    let mut output = String::new();
    output
}
