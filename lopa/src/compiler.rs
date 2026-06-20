use std::sync::{Arc, RwLock};

use lopa_core::vfs::Vfs;

#[salsa::db]
#[derive(Default)]
pub struct RootDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for RootDatabase {}

#[derive(Default)]
pub struct Compiler {
    db: RootDatabase,
    pub vfs: Arc<RwLock<Vfs>>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            db: Default::default(),
            vfs: Arc::new(RwLock::new(Vfs::default())),
        }
    }
}
