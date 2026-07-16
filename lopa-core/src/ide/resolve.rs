use itertools::Itertools;
use notify_rust::Notification;
use salsa::Accumulator;

use crate::{
    def::{
        Symbol, SymbolList,
        hir::{self, Module},
        mir::{self, BareFn, BareFnParam, BareFnParams, Type, TypeKind, TypeList},
    },
    ide::{Diagnostic, DiagnosticKind, DiagnosticLocation, ModuleDef, module_scope},
};

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub enum ResolveItemResult<'db> {
    Type(ModuleDef<'db>),
    Value(ModuleDef<'db>),
    Both {
        ty: ModuleDef<'db>,
        value: ModuleDef<'db>,
    },
}

#[salsa::tracked(returns(clone))]
pub fn resolve_module<'db>(db: &'db dyn salsa::Database, module: Module<'db>) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    for item in module.items(db).items(db).iter() {
        let hir::Item::Use(use_item) = item else {
            continue;
        };

        let mut ctx = ResolveUseTree {
            db,
            diagnostics: &mut diagnostics,
            use_item: *use_item,
            module,
        };

        if let Some(use_tree) = use_item.use_tree(db) {
            ctx.resolve(use_tree, SymbolList::new(db, []));
        }
    }
    diagnostics
}

fn resolve_path_cycle_result<'db>(
    _db: &'db dyn salsa::Database,
    _id: salsa::Id,
    _module: Module<'db>,
    _path: SymbolList,
) -> Option<ResolveItemResult<'db>> {
    None
}

#[salsa::tracked]
impl<'db> Module<'db> {
    #[salsa::tracked(returns(copy))]
    pub fn resolve_type_expr(
        self,
        db: &'db dyn salsa::Database,
        ty: hir::TypeExpr<'db>,
        generics: Option<mir::Generics<'db>>,
        self_ty: Option<Type<'db>>,
    ) -> Type<'db> {
        match ty.kind(db) {
            hir::TypeExprKind::Fn { params, output } => Type::new(
                db,
                TypeKind::BareFn(BareFn::new(
                    db,
                    BareFnParams::new(
                        db,
                        params
                            .params(db)
                            .iter()
                            .map(|p| {
                                BareFnParam::new(
                                    db,
                                    p.name(db),
                                    p.ty(db)
                                        .map(|t| self.resolve_type_expr(db, t, generics, self_ty))
                                        .unwrap_or_else(|| Type::new(db, TypeKind::Unit)),
                                )
                            })
                            .collect_vec(),
                    ),
                    output
                        .map(|t| self.resolve_type_expr(db, t, generics, self_ty))
                        .unwrap_or_else(|| Type::new(db, TypeKind::Unit)),
                )),
            ),
            hir::TypeExprKind::Tuple(types) => Type::new(
                db,
                TypeKind::Tuple(TypeList::new(
                    db,
                    types
                        .types(db)
                        .iter()
                        .map(|t| self.resolve_type_expr(db, *t, generics, self_ty))
                        .collect_vec(),
                )),
            ),
            hir::TypeExprKind::Path(_) => self.resolve_type_path(db, ty, generics, self_ty),
            hir::TypeExprKind::Dyn(bounds) => Type::new(
                db,
                TypeKind::Dyn(TypeList::new(
                    db,
                    bounds
                        .types(db)
                        .iter()
                        .map(|ty| self.resolve_type_path(db, *ty, generics, self_ty))
                        .collect_vec(),
                )),
            ),
            hir::TypeExprKind::Nilable(type_expr) => Type::new(
                db,
                TypeKind::Nilable(self.resolve_type_expr(db, type_expr, generics, self_ty)),
            ),
            hir::TypeExprKind::Paren(type_expr) => {
                self.resolve_type_expr(db, type_expr, generics, self_ty)
            }
            hir::TypeExprKind::Lit(lit_kind) => Type::new(db, TypeKind::Lit(lit_kind)),
            hir::TypeExprKind::Any => Type::new(db, TypeKind::Any),
            hir::TypeExprKind::Unit => Type::new(db, TypeKind::Unit),
            hir::TypeExprKind::Never => Type::new(db, TypeKind::Never),
            hir::TypeExprKind::SelfTy => {
                if let Some(self_ty) = self_ty {
                    self_ty
                } else {
                    Diagnostic {
                        message: "cannot find type `Self` in this scope".to_string(),
                        location: DiagnosticLocation::TypeExpr {
                            id: ty.id(db),
                            source: ty.source(db).get_pure(db),
                        },
                        kind: DiagnosticKind::TypeError,
                    }
                    .accumulate(db);
                    Type::new(db, TypeKind::Unknown)
                }
            }
        }
    }

    #[salsa::tracked(returns(copy))]
    pub fn resolve_type_path(
        self,
        db: &'db dyn salsa::Database,
        path_expr: hir::TypeExpr<'db>,
        generics: Option<mir::Generics<'db>>,
        self_ty: Option<Type<'db>>,
    ) -> Type<'db> {
        let hir::TypeExprKind::Path(path) = path_expr.kind(db) else {
            unreachable!();
        };
        if let [first] = path.segments(db)
            && let Some(generics) = generics
            && let Some(param) = generics.param(db, first.name(db))
        {
            return Type::new(db, TypeKind::Generic(param.name(db)));
        }

        //TODO: figure out what to do with stuff like
        //Foo<i32>::bar<Baz>
        // let generic_args = path.segments()

        let Some(item) = self.resolve_path_item(db, path.as_symbol_list(db)) else {
            Diagnostic {
                message: format!(
                    "cannot find type `{} in this scope",
                    path.segments(db).last().unwrap().name(db).value(db)
                ),
                location: DiagnosticLocation::TypeExpr {
                    id: path_expr.id(db),
                    source: path_expr.source(db).get_pure(db),
                },
                kind: DiagnosticKind::TypeError,
            }
            .accumulate(db);
            return Type::new(db, TypeKind::Unknown);
        };
        match item {
            ResolveItemResult::Type(ty) | ResolveItemResult::Both { ty, .. } => match ty {
                ModuleDef::Struct(item) => Type::new(
                    db,
                    TypeKind::Struct {
                        value: item,
                        generics: TypeList::new(db, []),
                    },
                ),
                ModuleDef::Enum(item) => Type::new(
                    db,
                    TypeKind::Enum {
                        value: item,
                        generics: TypeList::new(db, []),
                    },
                ),
                ModuleDef::Module(module) => {
                    Diagnostic {
                        message: format!(
                            "expected type, got module `{}`",
                            module.name(db).value(db)
                        ),
                        location: DiagnosticLocation::TypeExpr {
                            id: path_expr.id(db),
                            source: path_expr.source(db).get_pure(db),
                        },
                        kind: DiagnosticKind::TypeError,
                    }
                    .accumulate(db);
                    Type::new(db, TypeKind::Unknown)
                }
                ModuleDef::Function(function) => unreachable!(),
            },
            ResolveItemResult::Value(value) => match value {
                ModuleDef::Function(function) => {
                    Diagnostic {
                        message: format!(
                            "expected type, got function `{}`",
                            function.name(db).value(db)
                        ),
                        location: DiagnosticLocation::TypeExpr {
                            id: path_expr.id(db),
                            source: path_expr.source(db).get_pure(db),
                        },
                        kind: DiagnosticKind::TypeError,
                    }
                    .accumulate(db);
                    Type::new(db, TypeKind::Unknown)
                }
                ModuleDef::Struct(_) | ModuleDef::Enum(_) | ModuleDef::Module(_) => todo!(),
            },
        }
    }

    #[salsa::tracked(cycle_result=resolve_path_cycle_result, returns(copy))]
    pub fn resolve_path_item(
        self,
        db: &'db dyn salsa::Database,
        path: SymbolList,
    ) -> Option<ResolveItemResult<'db>> {
        fn resolve_path_inner<'db>(
            db: &'db dyn salsa::Database,
            module: Module<'db>,
            path: SymbolList,
        ) -> Option<ResolveItemResult<'db>> {
            let first = path.symbols(db).first()?;
            let mut current_item = match first.value(db) {
                "root" => {
                    ResolveItemResult::Type(ModuleDef::Module(module.root(db).root_module(db)?))
                }
                _ => module
                    .resolve_item_name(db, *path.symbols(db).first()?)
                    .or_else(|| {
                        let scope = module_scope(db, module);
                        for global_path in scope.global_imports() {
                            let mut global_path_clone = global_path.symbols(db).to_vec();
                            global_path_clone.push(*path.symbols(db).first()?);
                            if let Some(output) = module.resolve_path_item(db, *global_path) {
                                return Some(output);
                            } else {
                                continue;
                            }
                        }
                        None
                    })?,
            };
            for (id, segment) in path.symbols(db).iter().skip(1).enumerate() {
                match current_item.clone() {
                    ResolveItemResult::Type(module_def) => match module_def {
                        ModuleDef::Struct(item) => {
                            if id == path.symbols(db).len() - 1 {
                                return Some(ResolveItemResult::Type(ModuleDef::Struct(item)));
                            }
                            return None;
                        }
                        ModuleDef::Enum(item) => {
                            if id == path.symbols(db).len() - 1 {
                                return Some(ResolveItemResult::Type(ModuleDef::Enum(item)));
                            }
                            return None;
                        }
                        ModuleDef::Module(item) => {
                            if id == path.symbols(db).len() - 1 {
                                return Some(ResolveItemResult::Type(ModuleDef::Module(item)));
                            }
                            // if segment.value(db) == "self" {
                            //     continue;
                            // }

                            //TODO: public/private imports
                            let mod_path = SymbolList::new(db, [*segment]);
                            current_item = item.resolve_path_item(db, mod_path)?;
                        }
                        ModuleDef::Function(_) => unreachable!(),
                    },
                    ResolveItemResult::Value(module_def) => match module_def {
                        ModuleDef::Function(item) => {
                            if id == path.symbols(db).len() - 1 {
                                return Some(ResolveItemResult::Value(ModuleDef::Function(item)));
                            }
                            return None;
                        }
                        ModuleDef::Struct(_) | ModuleDef::Enum(_) | ModuleDef::Module(_) => {
                            unreachable!()
                        }
                    },
                    ResolveItemResult::Both { ty, value } => {
                        let ty = match ty {
                            ModuleDef::Struct(item) => {
                                if id == path.symbols(db).len() - 1 {
                                    Some(ModuleDef::Struct(item))
                                } else {
                                    None
                                }
                            }
                            ModuleDef::Enum(item) => {
                                if id == path.symbols(db).len() - 1 {
                                    Some(ModuleDef::Enum(item))
                                } else {
                                    None
                                }
                            }
                            ModuleDef::Module(item) => {
                                if id == path.symbols(db).len() - 1 {
                                    Some(ModuleDef::Module(item))
                                } else {
                                    // if segment.value(db) == "self" {
                                    //     continue;
                                    // }

                                    //TODO: public/private imports
                                    let mod_path = SymbolList::new(db, [*segment]);
                                    current_item = item.resolve_path_item(db, mod_path)?;
                                    None
                                }
                            }
                            ModuleDef::Function(_) => unreachable!(),
                        };
                        let value = match value {
                            ModuleDef::Function(item) => {
                                if id == path.symbols(db).len() - 1 {
                                    Some(ModuleDef::Function(item))
                                } else {
                                    None
                                }
                            }
                            ModuleDef::Struct(_) | ModuleDef::Enum(_) | ModuleDef::Module(_) => {
                                unreachable!()
                            }
                        };

                        match (ty, value) {
                            (Some(ty), Some(value)) => {
                                return Some(ResolveItemResult::Both { ty, value });
                            }
                            (Some(ty), None) => return Some(ResolveItemResult::Type(ty)),
                            (None, Some(value)) => return Some(ResolveItemResult::Value(value)),
                            _ => {}
                        }
                    }
                }
            }
            Some(current_item)
        }

        let scope = module_scope(db, self);
        let first = path.symbols(db).first()?;
        if let Some(scope_name) = scope
            .visible_type(*first)
            .or_else(|| scope.visible_value(*first))
        {
            let mut outer = scope_name.path(db).symbols(db).to_vec();
            outer.remove(outer.len() - 1);
            for symbol in path.symbols(db).iter() {
                outer.push(*symbol);
            }

            let outer = SymbolList::new(db, outer);
            if outer == path {
                resolve_path_inner(db, self, path)
            } else {
                resolve_path_inner(db, self, outer)
            }
        } else {
            resolve_path_inner(db, self, path)
        }
    }

    #[salsa::tracked(returns(copy))]
    pub fn resolve_item_name(
        self,
        db: &'db dyn salsa::Database,
        name: Symbol,
    ) -> Option<ResolveItemResult<'db>> {
        let scope = module_scope(db, self);
        let value = scope.value_item(name).cloned();
        let ty = scope.type_item(name).cloned();
        Some(match (value, ty) {
            (Some(value), Some(ty)) => ResolveItemResult::Both { ty, value },
            (Some(value), None) => ResolveItemResult::Value(value),
            (None, Some(ty)) => ResolveItemResult::Type(ty),
            _ => return None,
        })
    }
}

struct ResolveUseTree<'db, 'a> {
    db: &'db dyn salsa::Database,
    diagnostics: &'a mut Vec<Diagnostic>,
    use_item: hir::UseItem<'db>,
    module: hir::Module<'db>,
}

impl<'db, 'a> ResolveUseTree<'db, 'a> {
    fn push_diagnostic(&mut self, message: String, use_tree: hir::UseTree) {
        self.diagnostics.push(Diagnostic {
            message,
            location: DiagnosticLocation::UseTree {
                use_id: self.use_item.id(self.db),
                tree_id: use_tree.id(self.db),
            },
            kind: DiagnosticKind::ModuleError,
        });
    }

    fn resolve(&mut self, use_tree: hir::UseTree, path: SymbolList) {
        match use_tree.kind(self.db) {
            hir::UseTreeKind::Name(name) => {
                Notification::new().body("here").show().unwrap();
                let path = path.push(self.db, name);

                if self.module.resolve_path_item(self.db, path).is_none() {
                    self.push_diagnostic(
                        format!("unresolved import: `{}`", name.value(self.db)),
                        use_tree,
                    );
                }
            }
            hir::UseTreeKind::Path {
                name,
                use_tree: path_use_tree,
            } => {
                let path = path.push(self.db, name);
                if self.module.resolve_path_item(self.db, path).is_none() {
                    self.push_diagnostic(
                        format!("unresolved import `{}`", name.value(self.db)),
                        use_tree,
                    );
                }
                self.resolve(path_use_tree, path);
            }
            hir::UseTreeKind::Root {
                use_tree: root_use_tree,
            } => {
                let path = SymbolList::new(self.db, [Symbol::new(self.db, "root")]);
                self.resolve(root_use_tree, path);
            }
            hir::UseTreeKind::TreeList(use_tree_list) => {
                for item in use_tree_list.items(self.db) {
                    self.resolve(*item, path);
                }
            }
            hir::UseTreeKind::Super {
                use_tree: super_use_tree,
            } => {
                let Some(parent) = self.module.parent(self.db) else {
                    return;
                };
                self.module = parent;
                let path = parent.absolute_path(self.db);
                self.resolve(super_use_tree, path);
            }
            hir::UseTreeKind::Global | hir::UseTreeKind::SelfUse => {}
        }
    }
}
