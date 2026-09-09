use crate::{
    def::{
        Symbol, SymbolList,
        hir::{self, Elem, ElemKind, Enum, Function, Module, Struct, TypeExpr, UseItem},
    },
    ide::{self, Diagnostic, DiagnosticKind, DiagnosticLocation, ModuleDef, diagnostics},
};

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub enum ResolveItemError {
    NotFound,
    SelfNotFound,
    GotModule,
    GotFunction,
    Cycle,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub enum ResolveItem<'db> {
    Type(ModuleDef<'db>),
    Value(ModuleDef<'db>),
    Both {
        ty: ModuleDef<'db>,
        value: ModuleDef<'db>,
    },
}

#[salsa::tracked]
impl<'db> Struct<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.resolve_struct(self);
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> Module<'db> {
    pub fn path_map(self, db: &'db dyn salsa::Database) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, self);
        ctx.resolve_module(self);
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> Enum<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.resolve_enum(self);
        ctx.path_map()
    }
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn path_map(self, db: &'db dyn salsa::Database, module: Module<'db>) -> PathMap<'db> {
        let mut ctx = PathResolveCtx::new(db, module);
        ctx.resolve_fn(self);
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
                self.resolve_use_tree_path(path, use_item, use_tree);
            }
            hir::UseTreeKind::Path {
                name,
                use_tree: path_use_tree,
            } => {
                let path = path.push(self.db, name);
                self.resolve_use_tree_path(path, use_item, use_tree)?;
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
                self.resolve_use_tree_path(path, use_item, use_tree)?;
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

impl<'db> PathResolveCtx<'db> {
    fn new(db: &'db dyn salsa::Database, module: Module<'db>) -> Self {
        let mut ctx = Self {
            db,
            module,
            diagnostics: Default::default(),
            items: Default::default(),
            global_import_modules: Default::default(),
        };
        ctx.collect_globals();
        ctx
    }

    fn collect_globals(&mut self) {
        let scope = self.module.scope(self.db);
        for global in scope.global_imports.iter() {
            if let Ok(ResolveItem::Type(ty) | ResolveItem::Both { ty, .. }) =
                self.resolve_path(*global)
                && let ModuleDef::Module(module) = ty
            {
                self.global_import_modules.push(module);
            }
        }
    }

    fn canonicalize(&mut self, path: SymbolList) -> SymbolList {
        let scope = self.module.scope(self.db);
        let defs = self.module.defs(self.db);
        let mut path_vec = path.symbols(self.db).to_vec();
        loop {
            let first = path_vec[0];
            if defs.types.contains_key(&first) {
                break;
            }

            let Some(scope_name) = scope.types.get(&first) else {
                return path;
            };

            scope_name
                .path(self.db)
                .symbols(self.db)
                .iter()
                .rev()
                .skip(1)
                .for_each(|s| path_vec.insert(0, *s));
        }

        SymbolList::new(self.db, &path_vec)
    }

    fn resolve_path(
        &mut self,
        path: SymbolList,
    ) -> Result<ResolveItem<'db>, PathSegmentDiagnostic> {
        let canonicalized = self.canonicalize(path);
        let mut symbols = canonicalized.symbols(self.db).iter().enumerate();
        let mut current_item = match symbols.next().unwrap().1.value(self.db) {
            "root" => ResolveItem::Type(ModuleDef::Module(
                self.module.root(self.db).root_module(self.db).unwrap(),
            )),
            other => {
                let other = Symbol::new(self.db, other);
                let Some(item) = self.module.defs(self.db).resolve_item(other).or_else(|| {
                    for module in self.global_import_modules.iter() {
                        if let Some(item) = module.defs(self.db).resolve_item(other) {
                            return Some(item);
                        }
                    }
                    None
                }) else {
                    return Err(PathSegmentDiagnostic {
                        offset: 0,
                        message: format!("could not find `{}` in this scope", other.value(self.db)),
                    });
                };
                item
            }
        };

        for (id, symbol) in symbols {
            match current_item {
                ResolveItem::Type(ty) | ResolveItem::Both { ty, .. } => match ty {
                    ModuleDef::Struct(_) | ModuleDef::Enum(_) => {
                        return Err(PathSegmentDiagnostic {
                            offset: id,
                            message: "accessing struct/enum items not availible yet".to_string(),
                        });
                    }
                    ModuleDef::Module(item) => {
                        if symbol.value(self.db) == "super" {
                            current_item =
                                ResolveItem::Type(ModuleDef::Module(item.parent(self.db).unwrap()));
                            continue;
                        }
                        let Some(item) = item.defs(self.db).resolve_item(*symbol) else {
                            return Err(PathSegmentDiagnostic {
                                offset: id,
                                message: format!(
                                    "could not find `{}` in this scope",
                                    symbol.value(self.db)
                                ),
                            });
                        };
                        current_item = item;
                    }
                    ModuleDef::Function(_) => unreachable!(),
                },
                ResolveItem::Value(value) => match value {
                    ModuleDef::Function(function) => {
                        return Err(PathSegmentDiagnostic {
                            offset: id,
                            message: format!(
                                "could not find access members of function `{}`",
                                function.name(self.db).value(self.db)
                            ),
                        });
                    }
                    ModuleDef::Struct(_) | ModuleDef::Enum(_) | ModuleDef::Module(_) => {
                        unreachable!()
                    }
                },
            }
        }

        Ok(current_item)
    }

    fn resolve_use_tree_path(
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

    fn resolve_type_expr_path(&mut self, path: SymbolList, ty: TypeExpr<'db>) {
        match self.resolve_path(path) {
            Ok(item) => {
                self.items.insert(path, item);
            }
            Err(err) => {
                self.diagnostics.push(Diagnostic {
                    message: err.message,
                    location: DiagnosticLocation::PathSegment {
                        id: ty.id(self.db),
                        source: ty.source(self.db).body_map(self.db),
                        offset: err.offset,
                    },
                    kind: DiagnosticKind::TypeError,
                });
            }
        }
    }

    fn resolve_type_expr(&mut self, ty: TypeExpr<'db>) {
        match ty.kind(self.db) {
            hir::TypeExprKind::Path(path) => {
                let path = path.as_symbol_list(self.db);
                self.resolve_type_expr_path(path, ty);
            }
            hir::TypeExprKind::Tuple(tuple) => {
                for type_expr in tuple.types(self.db) {
                    self.resolve_type_expr(*type_expr);
                }
            }
            hir::TypeExprKind::Dyn(dyn_type) => {
                for type_expr in dyn_type.types(self.db) {
                    self.resolve_type_expr(*type_expr);
                }
            }
            hir::TypeExprKind::Nilable(type_expr) | hir::TypeExprKind::Paren(type_expr) => {
                self.resolve_type_expr(type_expr);
            }
            hir::TypeExprKind::Fn { params, output } => {
                for param in params.params(self.db) {
                    if let Some(ty) = param.ty(self.db) {
                        self.resolve_type_expr(ty);
                    }
                }
                if let Some(output) = output {
                    self.resolve_type_expr(output);
                }
            }
            hir::TypeExprKind::Any
            | hir::TypeExprKind::Unit
            | hir::TypeExprKind::Never
            | hir::TypeExprKind::SelfTy
            | hir::TypeExprKind::Lit(_) => {}
        }
    }

    fn resolve_elems_list(&mut self, elem_list: hir::ElemList<'db>) {
        for elem in elem_list.elems(self.db).iter() {
            let ElemKind::Field(field) = elem.kind(self.db) else {
                continue;
            };
            let Some(item_type_expr) = field.ty(self.db) else {
                continue;
            };

            match item_type_expr.kind(self.db) {
                hir::ItemTypeExprKind::TypeExpr(type_expr) => {
                    self.resolve_type_expr(type_expr);
                }
                hir::ItemTypeExprKind::Struct(_) => todo!(),
                hir::ItemTypeExprKind::Enum(_) => todo!(),
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

    fn resolve_struct(&mut self, item: hir::Struct<'db>) {
        self.resolve_elems_list(item.contents(self.db).elems);
    }

    fn resolve_enum(&mut self, item: hir::Enum<'db>) {
        self.resolve_elems_list(item.contents(self.db).elems);
    }

    fn resolve_module(&mut self, item: hir::Module<'db>) {
        for item in item.items(self.db).items(self.db) {
            match item {
                hir::Item::Use(item) => self.resolve_use_item(*item),
                hir::Item::Function(item) => {}
                hir::Item::Struct(item) => {}
                hir::Item::Enum(item) => {}
                hir::Item::Module(item) => {}
                hir::Item::Impl(item) => {}
            }
        }
    }

    fn resolve_fn(&mut self, item: hir::Function<'db>) {
        todo!()
    }

    fn path_map(self) -> PathMap<'db> {
        PathMap {
            items: self.items,
            diagnostics: self.diagnostics,
        }
    }
}

// #[salsa::tracked(returns(clone))]
// pub fn resolve_module<'db>(db: &'db dyn salsa::Database, module: Module<'db>) -> Vec<Diagnostic> {
//     let mut diagnostics = vec![];
//
//     for item in module.items(db).items(db).iter() {
//         let hir::Item::Use(use_item) = item else {
//             continue;
//         };
//
//         let mut ctx = ResolveUseTree {
//             db,
//             diagnostics: &mut diagnostics,
//             use_item: *use_item,
//             module,
//         };
//
//         if let Some(use_tree) = use_item.use_tree(db) {
//             ctx.resolve(use_tree, SymbolList::new(db, []));
//         }
//     }
//     diagnostics
// }
//
// fn resolve_path_cycle_result<'db>(
//     _db: &'db dyn salsa::Database,
//     _id: salsa::Id,
//     _module: Module<'db>,
//     _path: SymbolList,
// ) -> Option<ResolveItem<'db>> {
//     None
// }

// #[salsa::tracked]
// impl<'db> Module<'db> {
//     #[salsa::tracked(returns(copy))]
//     pub fn resolve_type_expr(
//         self,
//         db: &'db dyn salsa::Database,
//         ty: hir::TypeExpr<'db>,
//         generics: Option<mir::Generics<'db>>,
//         self_ty: Option<Type<'db>>,
//     ) -> Type<'db> {
//         match ty.kind(db) {
//             hir::TypeExprKind::Fn { params, output } => Type::new(
//                 db,
//                 TypeKind::BareFn(BareFn::new(
//                     db,
//                     BareFnParams::new(
//                         db,
//                         params
//                             .params(db)
//                             .iter()
//                             .map(|p| {
//                                 BareFnParam::new(
//                                     db,
//                                     p.name(db),
//                                     p.ty(db)
//                                         .map(|t| self.resolve_type_expr(db, t, generics, self_ty))
//                                         .unwrap_or_else(|| Type::new(db, TypeKind::Unit)),
//                                 )
//                             })
//                             .collect_vec(),
//                     ),
//                     output
//                         .map(|t| self.resolve_type_expr(db, t, generics, self_ty))
//                         .unwrap_or_else(|| Type::new(db, TypeKind::Unit)),
//                 )),
//             ),
//             hir::TypeExprKind::Tuple(types) => Type::new(
//                 db,
//                 TypeKind::Tuple(TypeList::new(
//                     db,
//                     types
//                         .types(db)
//                         .iter()
//                         .map(|t| self.resolve_type_expr(db, *t, generics, self_ty))
//                         .collect_vec(),
//                 )),
//             ),
//             hir::TypeExprKind::Path(_) => self.resolve_type_path(db, ty, generics, self_ty),
//             hir::TypeExprKind::Dyn(bounds) => Type::new(
//                 db,
//                 TypeKind::Dyn(TypeList::new(
//                     db,
//                     bounds
//                         .types(db)
//                         .iter()
//                         .map(|ty| self.resolve_type_path(db, *ty, generics, self_ty))
//                         .collect_vec(),
//                 )),
//             ),
//             hir::TypeExprKind::Nilable(type_expr) => Type::new(
//                 db,
//                 TypeKind::Nilable(self.resolve_type_expr(db, type_expr, generics, self_ty)),
//             ),
//             hir::TypeExprKind::Paren(type_expr) => {
//                 self.resolve_type_expr(db, type_expr, generics, self_ty)
//             }
//             hir::TypeExprKind::Lit(lit_kind) => Type::new(db, TypeKind::Lit(lit_kind)),
//             hir::TypeExprKind::Any => Type::new(db, TypeKind::Any),
//             hir::TypeExprKind::Unit => Type::new(db, TypeKind::Unit),
//             hir::TypeExprKind::Never => Type::new(db, TypeKind::Never),
//             hir::TypeExprKind::SelfTy => {
//                 if let Some(self_ty) = self_ty {
//                     self_ty
//                 } else {
//                     //TODO: this diagnostic needs to be moved somewhere else
//
//                     // Diagnostic {
//                     //     message: "cannot find type `Self` in this scope".to_string(),
//                     //     location: DiagnosticLocation::TypeExpr {
//                     //         id: ty.id(db),
//                     //         source: ty.source(db).get_pure(db),
//                     //     },
//                     //     kind: DiagnosticKind::TypeError,
//                     // }
//                     // .accumulate(db);
//                     Type::new(db, TypeKind::Unknown)
//                 }
//             }
//         }
//     }
//
//     #[salsa::tracked(returns(copy))]
//     pub fn resolve_type_path(
//         self,
//         db: &'db dyn salsa::Database,
//         path_expr: hir::TypeExpr<'db>,
//         generics: Option<mir::Generics<'db>>,
//         self_ty: Option<Type<'db>>,
//     ) -> Type<'db> {
//         let hir::TypeExprKind::Path(path) = path_expr.kind(db) else {
//             unreachable!();
//         };
//         if let [first] = path.segments(db)
//             && let Some(generics) = generics
//             && let Some(param) = generics.param(db, first.name(db))
//         {
//             return Type::new(db, TypeKind::Generic(param.name(db)));
//         }
//
//         //TODO: figure out what to do with stuff like
//         //Foo<i32>::bar<Baz>
//         // let generic_args = path.segments()
//
//         let Some(item) = self.resolve_path_item(db, path.as_symbol_list(db)) else {
//             Diagnostic {
//                 message: format!(
//                     "cannot find type `{} in this scope",
//                     path.segments(db).last().unwrap().name(db).value(db)
//                 ),
//                 location: DiagnosticLocation::TypeExpr {
//                     id: path_expr.id(db),
//                     source: path_expr.source(db).body_map(db),
//                 },
//                 kind: DiagnosticKind::TypeError,
//             }
//             .accumulate(db);
//             return Type::new(db, TypeKind::Unknown);
//         };
//         match item {
//             ResolveItem::Type(ty) | ResolveItem::Both { ty, .. } => match ty {
//                 ModuleDef::Struct(item) => Type::new(
//                     db,
//                     TypeKind::Struct {
//                         value: item,
//                         generics: TypeList::new(db, []),
//                     },
//                 ),
//                 ModuleDef::Enum(item) => Type::new(
//                     db,
//                     TypeKind::Enum {
//                         value: item,
//                         generics: TypeList::new(db, []),
//                     },
//                 ),
//                 ModuleDef::Module(module) => {
//                     Diagnostic {
//                         message: format!(
//                             "expected type, got module `{}`",
//                             module.name(db).value(db)
//                         ),
//                         location: DiagnosticLocation::TypeExpr {
//                             id: path_expr.id(db),
//                             source: path_expr.source(db).body_map(db),
//                         },
//                         kind: DiagnosticKind::TypeError,
//                     }
//                     .accumulate(db);
//                     Type::new(db, TypeKind::Unknown)
//                 }
//                 ModuleDef::Function(function) => unreachable!(),
//             },
//             ResolveItem::Value(value) => match value {
//                 ModuleDef::Function(function) => {
//                     Diagnostic {
//                         message: format!(
//                             "expected type, got function `{}`",
//                             function.name(db).value(db)
//                         ),
//                         location: DiagnosticLocation::TypeExpr {
//                             id: path_expr.id(db),
//                             source: path_expr.source(db).body_map(db),
//                         },
//                         kind: DiagnosticKind::TypeError,
//                     }
//                     .accumulate(db);
//                     Type::new(db, TypeKind::Unknown)
//                 }
//                 ModuleDef::Struct(_) | ModuleDef::Enum(_) | ModuleDef::Module(_) => todo!(),
//             },
//         }
//     }
//
//     #[salsa::tracked(cycle_result=resolve_path_cycle_result, returns(copy))]
//     pub fn resolve_path_item(
//         self,
//         db: &'db dyn salsa::Database,
//         path: SymbolList,
//     ) -> Option<ResolveItem<'db>> {
//         fn resolve_path_inner<'db>(
//             db: &'db dyn salsa::Database,
//             module: Module<'db>,
//             path: SymbolList,
//         ) -> Option<ResolveItem<'db>> {
//             let first = path.symbols(db).first()?;
//             let mut current_item = match first.value(db) {
//                 "root" => ResolveItem::Type(ModuleDef::Module(module.root(db).root_module(db)?)),
//                 _ => module
//                     .resolve_item_name(db, *path.symbols(db).first()?)
//                     .or_else(|| {
//                         let scope = module.scope(db);
//                         for global_path in scope.global_imports() {
//                             let mut global_path_clone = global_path.symbols(db).to_vec();
//                             global_path_clone.push(*path.symbols(db).first()?);
//                             if let Some(output) = module.resolve_path_item(db, *global_path) {
//                                 return Some(output);
//                             } else {
//                                 continue;
//                             }
//                         }
//                         None
//                     })?,
//             };
//             for (id, segment) in path.symbols(db).iter().skip(1).enumerate() {
//                 match current_item {
//                     ResolveItem::Type(module_def) => match module_def {
//                         ModuleDef::Struct(item) => {
//                             if id == path.symbols(db).len() - 1 {
//                                 return Some(ResolveItem::Type(ModuleDef::Struct(item)));
//                             }
//                             return None;
//                         }
//                         ModuleDef::Enum(item) => {
//                             if id == path.symbols(db).len() - 1 {
//                                 return Some(ResolveItem::Type(ModuleDef::Enum(item)));
//                             }
//                             return None;
//                         }
//                         ModuleDef::Module(item) => {
//                             if id == path.symbols(db).len() - 1 {
//                                 return Some(ResolveItem::Type(ModuleDef::Module(item)));
//                             }
//                             // if segment.value(db) == "self" {
//                             //     continue;
//                             // }
//
//                             //TODO: public/private imports
//                             current_item = item.resolve_item_name(db, *segment)?;
//                         }
//                         ModuleDef::Function(_) => unreachable!(),
//                     },
//                     ResolveItem::Value(module_def) => match module_def {
//                         ModuleDef::Function(item) => {
//                             if id == path.symbols(db).len() - 1 {
//                                 return Some(ResolveItem::Value(ModuleDef::Function(item)));
//                             }
//                             return None;
//                         }
//                         ModuleDef::Struct(_) | ModuleDef::Enum(_) | ModuleDef::Module(_) => {
//                             unreachable!()
//                         }
//                     },
//                     ResolveItem::Both { ty, value } => {
//                         let ty = match ty {
//                             ModuleDef::Struct(item) => {
//                                 if id == path.symbols(db).len() - 1 {
//                                     Some(ModuleDef::Struct(item))
//                                 } else {
//                                     None
//                                 }
//                             }
//                             ModuleDef::Enum(item) => {
//                                 if id == path.symbols(db).len() - 1 {
//                                     Some(ModuleDef::Enum(item))
//                                 } else {
//                                     None
//                                 }
//                             }
//                             ModuleDef::Module(item) => {
//                                 if id == path.symbols(db).len() - 1 {
//                                     Some(ModuleDef::Module(item))
//                                 } else {
//                                     // if segment.value(db) == "self" {
//                                     //     continue;
//                                     // }
//
//                                     //TODO: public/private imports
//                                     current_item = item.resolve_item_name(db, *segment)?;
//                                     None
//                                 }
//                             }
//                             ModuleDef::Function(_) => unreachable!(),
//                         };
//                         let value = match value {
//                             ModuleDef::Function(item) => {
//                                 if id == path.symbols(db).len() - 1 {
//                                     Some(ModuleDef::Function(item))
//                                 } else {
//                                     None
//                                 }
//                             }
//                             ModuleDef::Struct(_) | ModuleDef::Enum(_) | ModuleDef::Module(_) => {
//                                 unreachable!()
//                             }
//                         };
//
//                         match (ty, value) {
//                             (Some(ty), Some(value)) => {
//                                 return Some(ResolveItem::Both { ty, value });
//                             }
//                             (Some(ty), None) => return Some(ResolveItem::Type(ty)),
//                             (None, Some(value)) => return Some(ResolveItem::Value(value)),
//                             _ => {}
//                         }
//                     }
//                 }
//             }
//             Some(current_item)
//         }
//
//         let scope = self.scope(db);
//         let first = path.symbols(db).first()?;
//         if let Some(scope_name) = scope.r#type(*first).or_else(|| scope.value(*first)) {
//             let mut outer = scope_name.path(db).symbols(db).to_vec();
//             outer.remove(outer.len() - 1);
//             for symbol in path.symbols(db).iter() {
//                 outer.push(*symbol);
//             }
//
//             let outer = SymbolList::new(db, outer);
//             if outer == path {
//                 resolve_path_inner(db, self, path)
//             } else {
//                 resolve_path_inner(db, self, outer)
//             }
//         } else {
//             resolve_path_inner(db, self, path)
//         }
//     }
//
// }

// struct ResolveUseTree<'db, 'a> {
//     db: &'db dyn salsa::Database,
//     diagnostics: &'a mut Vec<Diagnostic>,
//     use_item: hir::UseItem<'db>,
//     module: hir::Module<'db>,
// }
//
// impl<'db, 'a> ResolveUseTree<'db, 'a> {
//     fn push_diagnostic(&mut self, message: String, use_tree: hir::UseTree) {
//         self.diagnostics.push(Diagnostic {
//             message,
//             location: DiagnosticLocation::UseTree {
//                 use_id: self.use_item.id(self.db),
//                 tree_id: use_tree.id(self.db),
//             },
//             kind: DiagnosticKind::ModuleError,
//         });
//     }
//
//     fn resolve(&mut self, use_tree: hir::UseTree, path: SymbolList) {
//         match use_tree.kind(self.db) {
//             hir::UseTreeKind::Name(name) => {
//                 let path = path.push(self.db, name);
//
//                 if self.module.resolve_path_item(self.db, path).is_none() {
//                     self.push_diagnostic(
//                         format!("unresolved import: `{}`", name.value(self.db)),
//                         use_tree,
//                     );
//                 }
//             }
//             hir::UseTreeKind::Path {
//                 name,
//                 use_tree: path_use_tree,
//             } => {
//                 let path = path.push(self.db, name);
//                 if self.module.resolve_path_item(self.db, path).is_none() {
//                     self.push_diagnostic(
//                         format!("unresolved import `{}`", name.value(self.db)),
//                         use_tree,
//                     );
//                 }
//                 self.resolve(path_use_tree, path);
//             }
//             hir::UseTreeKind::Root {
//                 use_tree: root_use_tree,
//             } => {
//                 let path = SymbolList::new(self.db, [Symbol::new(self.db, "root")]);
//                 self.resolve(root_use_tree, path);
//             }
//             hir::UseTreeKind::TreeList(use_tree_list) => {
//                 for item in use_tree_list.items(self.db) {
//                     self.resolve(*item, path);
//                 }
//             }
//             hir::UseTreeKind::Super {
//                 use_tree: super_use_tree,
//             } => {
//                 let Some(parent) = self.module.parent(self.db) else {
//                     return;
//                 };
//                 self.module = parent;
//                 let path = parent.absolute_path(self.db);
//                 self.resolve(super_use_tree, path);
//             }
//             hir::UseTreeKind::Global | hir::UseTreeKind::SelfUse => {}
//         }
//     }
// }

#[cfg(test)]
mod test {
    use std::{path::PathBuf, sync::Arc};

    use salsa::{Database, Setter};

    use crate::{
        def::hir::{self, Module, ModuleData},
        ide::{self, PathMap},
    };

    fn assert_module<'db>(db: &'db dyn salsa::Database, module: Module<'db>) {
        let path_map = module.path_map(db);
        if !path_map.diagnostics.is_empty() {
            panic!("{:#?}", path_map.diagnostics);
        }
        for item in module.items(db).items(db) {
            if let hir::Item::Module(item) = item {
                assert_module(db, *item);
            }
        }
    }

    fn assert_resolve_diagnostics(source: &str) {
        let mut db = salsa::DatabaseImpl::default();
        let root = ide::Root::new(&db, vec![], PathBuf::from("test"));
        let file = ide::File::new(
            &db,
            Arc::from(source),
            PathBuf::from("test/src/main.lopa"),
            root,
        );

        root.set_files(&mut db).to(vec![file]);

        for item in file.items(&db).items(&db) {
            if let hir::Item::Module(item) = item {
                assert_module(&db, *item);
            }
        }

        if let Some(item) = file.module(&db) {
            assert_module(&db, item);
        }
    }

    #[test]
    fn inner_mod() {
        assert_resolve_diagnostics(
            "mod test {
                mod bar {
                    struct X {}
                    enum Y {}
                }
                use bar::X;
                use bar::Y;
        }",
        );
    }

    #[test]
    fn single_super() {
        assert_resolve_diagnostics(
            "mod test {
                struct X {}
                mod foo {
                    use super::X;
                }
        }",
        );
    }

    #[test]
    fn multi_super() {
        assert_resolve_diagnostics(
            "mod test {
                struct X {}
                mod foo {
                    mod bar {
                        use super::super::X;
                    }
                }
        }",
        );
    }

    #[test]
    fn global_imports() {
        assert_resolve_diagnostics(
            "mod test {
                mod foo {
                    mod bar {
                        struct X {
                            idk: struct Y {}
                        }
                    }
                }

                use bar::Y;
                use foo::*;
        }",
        );
    }
}
