mod ast;
mod lexer;

use std::sync::{Arc, RwLock, atomic::AtomicUsize};

use dashmap::DashMap;
use petgraph::{adj::NodeIndex, graph::DiGraph};
use tower_lsp_server::ls_types::Uri;

#[derive(Default, Debug)]
struct Revision {
    verified_at: usize,
    changed_at: usize,
}

enum QueryKey {
    ContentOf(Uri),
    CstOf(Uri),
    NewlinesOf(Uri),
}

struct QueryContext {
    parent: Option<QueryKey>,
    db: Arc<Database>,
    dep_graph: Arc<DepGraph>,
}

struct Database {
    revisions: DashMap<QueryKey, Revision>,
    revision: AtomicUsize,

    //query caches
    content_input: DashMap<QueryKey, String>,
    // cst_query: DashMap<QueryKey, (Cst)>
}

struct DepGraph {
    graph: RwLock<DiGraph<QueryKey, ()>>,
    indices: DashMap<QueryKey, NodeIndex>,
}
