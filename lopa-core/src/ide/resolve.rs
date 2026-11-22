use crate::{
    common::PathNodeType,
    def::{
        Symbol, SymbolList,
        hir::{self, Enum, Function, HirNode, Item, Module, Struct, UseItem, UseTree},
    },
    hir_walker::{HirWalker, walk_expr, walk_pat, walk_type_expr},
    ide::{Diagnostic, DiagnosticKind, DiagnosticLocation, TypeModuleDef, ValueModuleDef},
};

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub enum ResolveItem<'db> {
    Type(TypeModuleDef<'db>),
    Value(ValueModuleDef<'db>),
    Both {
        ty: TypeModuleDef<'db>,
        value: ValueModuleDef<'db>,
    },
}

#[salsa::tracked]
impl<'db> Struct<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.walk_item(db, hir::Item::Struct(self), &mut ());
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> Module<'db> {
    pub fn path_map(self, db: &'db dyn salsa::Database) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, self);
        ctx.collect_global_imports();
        ctx.walk_item(db, hir::Item::Module(self), &mut ());
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> Enum<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.walk_item(db, hir::Item::Enum(self), &mut ());
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.walk_item(db, hir::Item::Function(self), &mut ());
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> UseItem<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.resolve_use_item(self);
        ctx.path_map()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
pub struct PathMap<'db> {
    pub items: indexmap::IndexMap<SymbolList, ResolveItem<'db>>,
    pub diagnostics: Vec<Diagnostic>,
}

struct ResolveUseTree<'db, 'a> {
    resolve_ctx: &'a mut PathResolveCtx<'db>,
    use_item: UseItem<'db>,
    module: Module<'db>,
}

impl<'db, 'a> std::ops::Deref for ResolveUseTree<'db, 'a> {
    type Target = PathResolveCtx<'db>;

    fn deref(&self) -> &Self::Target {
        self.resolve_ctx
    }
}

impl<'db, 'a> std::ops::DerefMut for ResolveUseTree<'db, 'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.resolve_ctx
    }
}

impl<'db, 'a> ResolveUseTree<'db, 'a> {
    fn resolve(&mut self) -> Option<()> {
        self.resolve_inner(
            self.use_item.use_tree(self.db)?,
            SymbolList::new(self.resolve_ctx.db, []),
        )
    }
    fn resolve_inner(&mut self, use_tree: hir::UseTree<'db>, path: SymbolList) -> Option<()> {
        let use_item = self.use_item;
        match use_tree.kind(self.db) {
            hir::UseTreeKind::Name(name) => {
                let path = path.push(self.db, name);
                self.insert_use_tree_path(path, use_item, use_tree);
            }
            hir::UseTreeKind::Path {
                name,
                use_tree: path_use_tree,
            } => {
                let path = path.push(self.db, name);
                self.insert_use_tree_path(path, use_item, use_tree)?;
                self.resolve_inner(path_use_tree, path)?;
            }
            //NOTE: super can only be used in start positions, so foo::bar::super is not valid
            hir::UseTreeKind::Super { use_tree } => {
                let parent = self.module.parent(self.db).unwrap();
                self.module = parent;
                let path = parent.absolute_path(self.db);
                self.resolve_inner(use_tree, path)?;
            }
            hir::UseTreeKind::Root { use_tree } => {
                let path = SymbolList::new(self.db, [Symbol::new(self.db, "root")]);
                self.resolve_inner(use_tree, path)?;
            }
            hir::UseTreeKind::TreeList(use_tree_list) => {
                for item in use_tree_list.items(self.db).iter() {
                    self.resolve_inner(*item, path);
                }
            }
            hir::UseTreeKind::SelfUse => {
                self.insert_use_tree_path(path, use_item, use_tree)?;
            }
            hir::UseTreeKind::Global => {}
        }
        Some(())
    }
}

struct PathSegmentDiagnostic {
    offset: usize,
    message: String,
}

struct PathResolveCtx<'db> {
    db: &'db dyn salsa::Database,
    module: Module<'db>,
    diagnostics: Vec<Diagnostic>,
    items: indexmap::IndexMap<SymbolList, ResolveItem<'db>>,
    global_import_modules: Vec<Module<'db>>,
}

impl<'db> HirWalker<'db> for PathResolveCtx<'db> {
    type Context = ();
    fn walk_expr(&mut self, db: &'db dyn salsa::Database, expr: hir::Expr<'db>, ctx: &mut ()) {
        match expr.kind(db) {
            hir::ExprKind::Path(path) => {
                self.insert_path(path.as_symbol_list(db), expr, PathNodeType::Expr);
            }
            hir::ExprKind::Record { path, .. } => {
                self.insert_path(path.as_symbol_list(self.db), expr, PathNodeType::Record);
            }
            _ => {}
        }
        walk_expr(db, self, expr, ctx);
    }

    fn walk_type_expr(
        &mut self,
        db: &'db dyn salsa::Database,
        type_expr: hir::TypeExpr<'db>,
        ctx: &mut (),
    ) {
        if let hir::TypeExprKind::Path(path) = type_expr.kind(db) {
            self.insert_path(path.as_symbol_list(db), type_expr, PathNodeType::Type);
        }
        walk_type_expr(db, self, type_expr, ctx);
    }

    fn walk_pat(&mut self, db: &'db dyn salsa::Database, pat: hir::Pat<'db>, ctx: &mut ()) {
        if let hir::PatKind::Path(path) = pat.kind(db) {
            self.insert_path(path.as_symbol_list(db), pat, PathNodeType::Pat);
        }
        walk_pat(db, self, pat, ctx);
    }
}

impl<'db> PathResolveCtx<'db> {
    fn new(db: &'db dyn salsa::Database, module: Module<'db>) -> Self {
        Self {
            db,
            module,
            diagnostics: Default::default(),
            items: Default::default(),
            global_import_modules: Default::default(),
        }
    }

    fn collect_global_imports(&mut self) {
        // let scope = self.module.scope_names(self.db);
        // for global in scope.global_imports.iter() {
        //     if let Ok(ResolveItem::Type(ty) | ResolveItem::Both { ty, .. }) =
        //         self.resolve_path(*global)
        //         && let TypeModuleDef::Module(module) = ty
        //     {
        //         self.global_import_modules.push(module);
        //     }
        // }
    }

    fn canonicalize(&mut self, path: SymbolList) -> SymbolList {
        todo!()
        // let scope = self.module.scope_names(self.db);
        // let defs = self.module.defs(self.db);
        // let mut path_vec = path.symbols(self.db).to_vec();
        // loop {
        //     let first = path_vec[0];
        //     if defs.types.contains_key(&first) {
        //         break;
        //     }
        //
        //     let Some(scope_name) = scope.types.get(&first) else {
        //         return path;
        //     };
        //
        //     let symbols = scope_name
        //         .path(self.db)
        //         .symbols(self.db)
        //         .iter()
        //         .rev()
        //         .skip(1);
        //     if symbols.len() == 0 {
        //         break;
        //     }
        //     for symbol in symbols {
        //         path_vec.insert(0, *symbol);
        //     }
        // }

        // SymbolList::new(self.db, &path_vec)
    }

    fn insert_path<H: HirNode<'db>>(&mut self, path: SymbolList, node: H, node_type: PathNodeType) {
        match self.resolve_path(path) {
            Ok(item) => {
                self.items.insert(path, item);
            }
            Err(err) => {
                // salsa::attach(self.db, || {
                //     panic!("{:?} {:?}", path, node_type);
                // });
                self.diagnostics.push(Diagnostic {
                    message: err.message,
                    location: DiagnosticLocation::PathSegment {
                        id: node.hir_id(self.db),
                        source: node.body_map_source(self.db).body_map(self.db),
                        offset: err.offset,
                        node_type,
                    },
                    kind: DiagnosticKind::TypeError,
                });
            }
        }
    }

    fn resolve_path(
        &mut self,
        path: SymbolList,
    ) -> Result<ResolveItem<'db>, PathSegmentDiagnostic> {
        // let defs = self.module.defs(self.db);
        // let scope_names = self.module.scope_names(self.db);
        todo!()
    }

    // fn resolve_path(
    //     &mut self,
    //     path: SymbolList,
    // ) -> Result<ResolveItem<'db>, PathSegmentDiagnostic> {
    //     let canonicalized = self.canonicalize(path);
    //     let mut symbols = canonicalized.symbols(self.db).iter().enumerate();
    //     let mut current_item = match symbols.next().unwrap().1.value(self.db) {
    //         "root" => ResolveItem::Type(TypeModuleDef::Module(
    //             self.module.root(self.db).root_module(self.db).unwrap(),
    //         )),
    //         other => {
    //             let other = Symbol::new(self.db, other);
    //             let Some(item) = self.module.defs(self.db).resolve_item(other).or_else(|| {
    //                 for module in self.global_import_modules.iter() {
    //                     if let Some(item) = module.defs(self.db).resolve_item(other) {
    //                         return Some(item);
    //                     }
    //                 }
    //                 None
    //             }) else {
    //                 return Err(PathSegmentDiagnostic {
    //                     offset: 0,
    //                     message: format!("could not find `{}` in this scope", other.value(self.db)),
    //                 });
    //             };
    //             item
    //         }
    //     };
    //
    //     for (id, symbol) in symbols {
    //         match current_item {
    //             ResolveItem::Type(ty) | ResolveItem::Both { ty, .. } => match ty {
    //                 TypeModuleDef::Struct(_)
    //                 | TypeModuleDef::Enum(_)
    //                 | TypeModuleDef::Class(_)
    //                 | TypeModuleDef::Trait(_) => {
    //                     return Err(PathSegmentDiagnostic {
    //                         offset: id,
    //                         message: "accessing this item not availible yet".to_string(),
    //                     });
    //                 }
    //                 TypeModuleDef::Module(item) => {
    //                     if symbol.value(self.db) == "super" {
    //                         current_item = ResolveItem::Type(TypeModuleDef::Module(
    //                             item.parent(self.db).unwrap(),
    //                         ));
    //                         continue;
    //                     }
    //                     let Some(item) = item.defs(self.db).resolve_item(*symbol) else {
    //                         return Err(PathSegmentDiagnostic {
    //                             offset: id,
    //                             message: format!(
    //                                 "could not find `{}` in this scope",
    //                                 symbol.value(self.db)
    //                             ),
    //                         });
    //                     };
    //                     current_item = item;
    //                 }
    //             },
    //             ResolveItem::Value(value) => match value {
    //                 ValueModuleDef::Function(function) => {
    //                     return Err(PathSegmentDiagnostic {
    //                         offset: id,
    //                         message: format!(
    //                             "could not find access members of function `{}`",
    //                             function.name(self.db).value(self.db)
    //                         ),
    //                     });
    //                 }
    //             },
    //         }
    //     }
    //
    //     Ok(current_item)
    // }

    fn insert_use_tree_path(
        &mut self,
        path: SymbolList,
        use_item: hir::UseItem<'db>,
        use_tree: hir::UseTree<'db>,
    ) -> Option<()> {
        match self.resolve_path(path) {
            Ok(item) => {
                self.items.insert(path, item);
                Some(())
            }
            Err(err) => {
                self.diagnostics.push(Diagnostic {
                    message: err.message,
                    location: DiagnosticLocation::UseTree {
                        use_id: use_item.id(self.db),
                        tree_id: use_tree.id(self.db),
                    },
                    kind: DiagnosticKind::TypeError,
                });
                None
            }
        }
    }

    fn resolve_use_item(&mut self, use_item: hir::UseItem<'db>) {
        let module = self.module;
        ResolveUseTree {
            resolve_ctx: self,
            use_item,
            module,
        }
        .resolve();
    }

    fn path_map(self) -> PathMap<'db> {
        PathMap {
            items: self.items,
            diagnostics: self.diagnostics,
        }
    }
}

// #[cfg(test)]
// mod test {
//     use std::{path::PathBuf, sync::Arc};
//
//     use salsa::Setter;
//
//     use crate::{
//         def::hir::{self, Function, Module},
//         ide::{self, DefMap, Root},
//     };
//
//     fn assert_module<'db>(db: &'db dyn salsa::Database, module: Module<'db>) {
//         let path_map = module.path_map(db);
//         if !path_map.diagnostics.is_empty() {
//             panic!("{:#?}", path_map.diagnostics);
//         }
//         for item in module.items(db).items(db) {
//             match item {
//                 hir::Item::Function(item) => assert_function(db, *item, module),
//                 hir::Item::Module(item) => assert_module(db, *item),
//                 hir::Item::Impl(item) => {
//                     item.contents(db);
//                 }
//                 _ => {}
//             }
//         }
//     }
//
//     fn assert_function<'db>(
//         db: &'db dyn salsa::Database,
//         func: Function<'db>,
//         module: Module<'db>,
//     ) {
//         let path_map = func.path_map(db, module);
//         if !path_map.diagnostics.is_empty() {
//             panic!("{:#?}", path_map.diagnostics);
//         }
//     }
//
//     fn assert_resolve_diagnostics(source: &str) {
//         let mut db = salsa::DatabaseImpl::default();
//         let root = ide::Root::new(&db, vec![], PathBuf::from("test"));
//         let file = ide::File::new(
//             &db,
//             Arc::from(source),
//             PathBuf::from("test/src/main.lopa"),
//             root,
//         );
//
//         root.set_files(&mut db).to(vec![file]);
//
//         if let Some(item) = file.module(&db) {
//             assert_module(&db, item);
//         }
//     }
//
//     #[test]
//     fn inner_mod() {
//         assert_resolve_diagnostics(
//             "mod test {
//                 mod bar {
//                     struct X {}
//                     enum Y {}
//                 }
//                 use bar::X;
//                 use bar::Y;
//         }",
//         );
//     }
//
//     #[test]
//     fn single_super() {
//         assert_resolve_diagnostics(
//             "mod test {
//                 struct X {}
//                 mod foo {
//                     use super::X;
//                 }
//         }",
//         );
//     }
//
//     #[test]
//     fn multi_super() {
//         assert_resolve_diagnostics(
//             "mod test {
//                 struct X {}
//                 mod foo {
//                     mod bar {
//                         use super::super::X;
//                     }
//                 }
//         }",
//         );
//     }
//
//     #[test]
//     fn inner_items() {
//         assert_resolve_diagnostics(
//             "mod test {
//                 mod foo {
//                     struct X {
//                         idk: struct Y {};
//                     }
//                 }
//                 use foo::X;
//                 use foo::Y;
//
//                 impl X for Y {
//                     fn a(){}
//                     fn b() {}
//                 }
//         }",
//         )
//     }
//
//     #[test]
//     fn global_imports() {
//         assert_resolve_diagnostics(
//             "mod test {
//                 mod foo {
//                     mod bar {
//                         struct X {
//                             idk: struct Y {};
//                         }
//                     }
//                 }
//
//                 use bar::Y;
//                 use foo::*;
//         }",
//         );
//     }
//
//     #[test]
//     fn fn_exprs() {
//         assert_resolve_diagnostics(
//             "mod test {
//                 mod A {
//                     mod B {
//                         struct C {}
//                     }
//                 }
//
//                 fn my_func() {
//                 }
//         }",
//         );
//     }
// }
// let _ = (
//     A::B::C,
//     A::B::C,
//     { if A::B::C {
//                 } {} }
// );

/*
Name resolution notes.

How I imagine it should work:
1. Collect defs (values + types) for module
2. Collect scope names per module:
    Scope names are all names accessible through a module, including `use` statements.
3. Find a local scope of a module, derived from 2: IndexMap<Symbol, Item>
4. Collect all paths that are referenced in a module, and check if they are valid (via PathMap)

Main difficulty lies in 3., as it seems like this step already requried name resolution, which should only happen at 4.
Solution: reuse PathMap for 3. and 4.?


*/
