
use std::sync::{Arc, RwLock, atomic::AtomicUsize};

use dashmap::DashMap;
use petgraph::{adj::NodeIndex, graph::DiGraph};
use tower_lsp_server::ls_types::Uri;

use crate::lsp::parser::Cst;

#[derive(Default, Debug)]
pub struct Revision {
    verified_at: usize,
    changed_at: usize,
}

pub enum QueryKey {
    ContentOf(Uri),
    CstOf(Uri),
    NewlinesOf(Uri),
}

pub struct QueryContext {
    parent: Option<QueryKey>,
    db: Arc<Database>,
    dep_graph: Arc<DepGraph>,
}

pub struct Database {
    revisions: DashMap<QueryKey, Revision>,
    revision: AtomicUsize,

    //query caches
    content_input: DashMap<QueryKey, String>,
    cst_query: DashMap<QueryKey, ()>,
    // newlines_
}

pub struct DepGraph {
    graph: RwLock<DiGraph<QueryKey, ()>>,
    indices: DashMap<QueryKey, NodeIndex>,
}
