use std::sync::{Arc, RwLock};

use lopa_core::vfs::Vfs;
use salsa::Database;

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

    fn attach<R>(&self, op: impl FnOnce(&RootDatabase) -> R) -> R
    where
        Self: Sized,
    {
        self.db.attach(op)
    }
}
