use std::{path::Path, sync::Arc};

use indexmap::map::Entry;
use itertools::Itertools;
use notify_rust::Notification;
use ustr::Ustr;

use crate::{
    def::{
        ir::{self, ExprId, Local},
        lower,
        scope::{self, ScopeId},
    },
    ide::{self, diagnostics::Diagnostic},
    parsing::ast,
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

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, salsa::Update)]
pub enum ResolveItemResult<'db> {
    Function(ir::Function<'db>),
    Struct(ir::Struct<'db>),
    Module(ide::File),
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
//
#[salsa::tracked]
pub fn resolve_item_name<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    name: Ustr,
) -> Option<ResolveItemResult<'db>> {
    let module_scope = scope::module_scope(db, file);

    Some(if let Some(value) = module_scope.resolve_value(&name) {
        match value {
            ir::ModuleValueDef::Function(function) => ResolveItemResult::Function(*function),
        }
    } else if let Some(ty) = module_scope.resolve_type(&name) {
        match ty {
            ir::ModuleTypeDef::Struct(strct) => ResolveItemResult::Struct(*strct),
            ir::ModuleTypeDef::Module(file) => ResolveItemResult::Module(*file),
        }
    } else {
        return None;
    })
}

#[salsa::tracked]
pub fn resolve_path<'db>(
    db: &'db dyn salsa::Database,
    file: ide::File,
    path: ir::Path,
) -> Option<ResolveItemResult<'db>> {
    fn resolve_path_inner<'db>(
        db: &'db dyn salsa::Database,
        file: ide::File,
        path: ir::Path,
    ) -> Option<ResolveItemResult<'db>> {
        let first = *path.0.first()?;
        let mut current_item = match first.as_str() {
            "root" => ResolveItemResult::Module(ide::root_module(db, file.source_root(db))?),
            _ => resolve_item_name(db, file, *path.0.first()?).or_else(|| {
                let scope = scope::module_scope(db, file);
                for global_path in scope.global_imports() {
                    let mut global_path = global_path.clone();
                    global_path.0.push(*path.0.first()?);
                    let Some(output) = resolve_path(db, file, global_path.clone()) else {
                        continue;
                    };
                    return Some(output);
                }
                None
            })?,
        };
        for (id, segment) in path.0.iter().skip(1).enumerate() {
            match current_item {
                ResolveItemResult::Function(function) => {
                    if id == path.0.len() - 1 {
                        return Some(ResolveItemResult::Function(function));
                    }
                    return None;
                }
                ResolveItemResult::Struct(strct) => {
                    if id == path.0.len() - 1 {
                        return Some(ResolveItemResult::Struct(strct));
                    }
                    return None;
                }
                ResolveItemResult::Module(file) => {
                    if id == path.0.len() - 1 {
                        return Some(ResolveItemResult::Module(file));
                    }
                    if segment == "self" {
                        continue;
                    }
                    let mut module_path = ide::module_path(db, file);
                    module_path.0.push(*segment);
                    current_item = resolve_path(db, file, ir::Path(vec![*segment]))?;
                }
            }
        }
        Some(current_item)
    }

    let module_scope = scope::module_scope(db, file);
    let first = path.0.first()?;
    if let Some(outer) = module_scope.resolve_name(first) {
        let mut outer = outer.clone();
        outer.0.remove(outer.0.len() - 1);
        outer.0.append(&mut path.0.clone());
        if outer == path {
            resolve_path_inner(db, file, path)
        } else {
            resolve_path(db, file, outer)
        }
    } else {
        resolve_path_inner(db, file, path)
    }
}

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
            ir::ModuleValueDef::Function(function) => {
                return Some(ResolveResult::Function(*function));
            }
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
//
