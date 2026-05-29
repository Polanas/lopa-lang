use std::sync::Arc;

use indexmap::map::Entry;
use itertools::Itertools;
use ustr::Ustr;

use crate::{
    def::{
        ir::{self, ExprId, Local},
        scope::{self, ScopeId},
    },
    ide,
    ustr_hash::UstrIndexMap,
};
//
// #[derive(PartialEq, Eq, Clone, salsa::Update)]
// pub struct Resolver<'db> {
//     scopes: Vec<Scope<'db>>,
//     file_scope: scope::FileScope<'db>,
// }
//
// #[salsa::tracked(returns(ref))]
// pub fn resolver_for_top_level<'db>(db: &'db dyn salsa::Database, file: ide::File) -> Resolver<'db> {
//     let scopes = scope::file_scope(db, file);
//     Resolver {
//         scopes: vec![],
//         file_scope: scopes.clone(),
//     }
// }
//
// #[salsa::tracked(returns(ref))]
// pub fn resolver_for_expr<'db>(
//     db: &'db dyn salsa::Database,
//     owner: ir::Function<'db>,
//     expr_id: ExprId,
// ) -> Resolver<'db> {
//     let scopes = scope::expr_scopes(db, owner);
//     resolver_for_scope(db, owner, scopes.scope_for_expr(expr_id)).clone()
// }
//
// #[salsa::tracked(returns(ref))]
// pub fn resolver_for_scope<'db>(
//     db: &'db dyn salsa::Database,
//     owner: ir::Function<'db>,
//     scope_id: Option<ScopeId>,
// ) -> Resolver<'db> {
//     let file_scope = scope::file_scope(db, owner.file(db));
//     let scopes = scope::expr_scopes(db, owner);
//     let scope_chain = scopes.scope_chain(scope_id).collect_vec();
//
//     let mut resolver = Resolver {
//         scopes: Vec::with_capacity(scope_chain.len()),
//         file_scope: file_scope.clone(),
//     };
//     for scope in scope_chain.into_iter().rev() {
//         resolver = resolver.push_expr_scope(owner, scopes.clone(), scope);
//     }
//
//     resolver
// }
//
// #[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
// struct Scope<'db> {
//     owner: ir::Function<'db>,
//     expr_scopes: Arc<scope::ExprScopes>,
//     scope_id: scope::ScopeId,
// }
//
#[derive(Default)]
pub struct ScopeNames<'db> {
    names: UstrIndexMap<ResolveResult<'db>>,
}
//
// impl<'db> ScopeNames<'db> {
//     fn add(&mut self, name: &Ustr, def: ResolveResult<'db>) {
//         match self.names.entry((*name).into()) {
//             Entry::Occupied(_) => {}
//             Entry::Vacant(entry) => {
//                 entry.insert(def);
//             }
//         }
//         // //TODO: should this only insert on vacant entries?
//         // self.names.insert(name.precomputed_hash(), def);
//     }
// }
//
#[derive(Debug)]
pub enum ResolveResult<'db> {
    Local(ir::Local<'db>),
    Function(ir::Function<'db>),
    Struct(ir::Struct<'db>),
}
//
// impl<'db> Resolver<'db> {
//     pub fn names_in_scope(&self) -> UstrIndexMap<ResolveResult<'_>> {
//         let mut map = ScopeNames::default();
//
//         for scope in self.scopes() {
//             for expr_scope in scope.expr_scopes.scope_chain(Some(scope.scope_id)) {
//                 let entries = scope.expr_scopes.entries(expr_scope);
//                 for entry in entries {
//                     if let Some(owner) = self.body_owner() {
//                         map.add(
//                             &entry.name(),
//                             ResolveResult::Local(Local {
//                                 parent: owner,
//                                 pattern_id: entry.pattern(),
//                             }),
//                         );
//                     }
//                 }
//             }
//         }
//
//         //TODO: finish
//         // for (name, file_def) in self.file_scope.values() {
//         //     match file_def {
//         //         ir::FileDef::Function(function) => {
//         //             map.add(&name.0, ResolveResult::Function(*function));
//         //         }
//         //     }
//         // }
//
//         map.names
//     }
//
//     pub fn resolve_name(&self, name: &Ustr) -> Option<ResolveResult<'_>> {
//         for scope in self.scopes() {
//             let entry = scope
//                 .expr_scopes
//                 .resolve_name_in_scope(scope.scope_id, name);
//
//             if let (Some(entry), Some(owner)) = (entry, self.body_owner()) {
//                 return Some(ResolveResult::Local(Local {
//                     parent: owner,
//                     pattern_id: entry.pattern(),
//                 }));
//             }
//         }
//
//         //TODO: finish
//         // if let Some(result) = self.file_scope.resolve_name(name) {
//         //     match result {
//         //         ir::FileDef::Function(function) => return Some(ResolveResult::Function(*function)),
//         //     }
//         // }
//
//         None
//     }
//
//     pub fn body_owner(&self) -> Option<ir::Function<'_>> {
//         self.scopes().next().map(|s| s.owner)
//     }
//
//     fn scopes(&self) -> impl Iterator<Item = &Scope<'_>> {
//         self.scopes.iter().rev()
//     }
//
//     fn push_expr_scope(
//         self,
//         owner: ir::Function<'db>,
//         expr_scopes: Arc<scope::ExprScopes>,
//         scope_id: scope::ScopeId,
//     ) -> Self {
//         self.push_scope(Scope {
//             owner,
//             expr_scopes,
//             scope_id,
//         })
//     }
//     fn push_scope(mut self, scope: Scope<'db>) -> Self {
//         self.scopes.push(scope);
//         self
//     }
// }
//
// #[cfg(test)]
// mod test {
//     use std::sync::{Arc, RwLock};
//
//     use salsa::{Database, DatabaseImpl};
//
//     use crate::ide::{self, FileContent};
//
//     #[test]
//     fn names_in_scope() {
//         //TODO: write an acutal test
//         DatabaseImpl::default().attach(|db| {
//             let input = ide::File::new(
//                 db,
//                 Arc::new(RwLock::new(FileContent::new(String::from(
//                     "fn main() {
//                     let x = 1;
//                     if true {
//                         let y = 1;
//                     }
//                 }",
//                 )))),
//             );
//             let ir = ide::lower_file(db, input);
//             dbg!(ir.functions(db));
//         });
//     }
// }

pub fn resolve_name_for_expr<'db>(
    db: &'db dyn salsa::Database,
    expr: ExprId,
    func: ir::Function<'db>,
    name: &Ustr,
) -> Option<ResolveResult<'db>> {
    let scopes = scope::expr_scopes(db, func);
    let expr_scope = scopes.scope_for_expr(expr)?;
    if let Some(entry) = scopes.resolve_name_in_scope(expr_scope, &name) {
        return Some(ResolveResult::Local(Local {
            parent: func,
            pattern_id: entry.pattern(),
        }));
    }
    let module_scope = scope::module_scope(db, func.file(db));

    if let Some(result) = module_scope.resolve_value(name) {
        match result {
            ir::ModuleDef::Function(function) => return Some(ResolveResult::Function(*function)),
            ir::ModuleDef::Struct(strct) => todo!(),
        }
    }
    None
}

// pub fn resolve_name(&self, name: &Ustr) -> Option<ResolveResult<'_>> {
//     for scope in self.scopes() {
//         let entry = scope
//             .expr_scopes
//             .resolve_name_in_scope(scope.scope_id, name);
//
//         if let (Some(entry), Some(owner)) = (entry, self.body_owner()) {
//             return Some(ResolveResult::Local(Local {
//                 parent: owner,
//                 pattern_id: entry.pattern(),
//             }));
//         }
//     }
//
//     //TODO: finish
//     // if let Some(result) = self.file_scope.resolve_name(name) {
//     //     match result {
//     //         ir::FileDef::Function(function) => return Some(ResolveResult::Function(*function)),
//     //     }
//     // }
//
//     None
// }
