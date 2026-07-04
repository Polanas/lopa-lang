use crate::{
    ide::{self, Diagnostic, File, RenderedDiagnostic},
    vfs::FileContent,
};

#[salsa::db]
#[derive(Default, Clone)]
pub struct MyDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for MyDatabase {}

pub struct Analysis {
    pub db: MyDatabase,
}

impl Analysis {
    pub fn new() -> Self {
        let db = MyDatabase::default();
        Self { db }
    }

    pub fn format(&self, file: File) -> String {
        todo!()
        // format::format_file(&self.db, file)
    }

    pub fn diagnostics(&self, file: File) -> Vec<RenderedDiagnostic> {
        file.rendered_diagnostics(&self.db)
    }
}

impl Default for Analysis {
    fn default() -> Self {
        Self::new()
    }
}
