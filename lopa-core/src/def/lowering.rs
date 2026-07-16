use std::sync::Arc;

use itertools::Itertools as _;
use notify_rust::Notification;

use crate::{
    def::{AstIdMap, ContentsMap, ItemsMap, Symbol, UseTreeMap, body_map::BodyMap, hir::*},
    ide::File,
    parsing::{self},
};

struct BodyCtx<'db, 's> {
    db: &'db dyn salsa::Database,
    source: &'s str,
    map: BodyMap,
    id_source: Option<IdSource<'db>>,
    file: File,
}

impl<'db, 's> BodyCtx<'db, 's> {
    fn new(db: &'db dyn salsa::Database, source: &'s str, file: File) -> Self {
        Self {
            db,
            source,
            file,
            map: Default::default(),
            id_source: None,
        }
    }

    fn fn_item(mut self, item: Function<'db>) -> Option<FunctionBody<'db>> {
        let ast_map = self.file.ast_map(self.db);
        let parse = self.file.parse(self.db);
        let ast_item = parse.cast::<parsing::FnItem>(self.db, ast_map.get(item.id(self.db))?)?;

        self.id_source = Some(IdSource::BodySource(BodyMapSource::Function(item)));

        let params = ast_item
            .params()
            .map(|p| p.params())
            .into_iter()
            .flatten()
            .filter_map(|p| self.fn_param(p))
            .collect_vec();

        let body_expr = ast_item.body().map(|b| self.block_expr(b))?;

        Some(FunctionBody {
            body_map: self.map.into(),
            body_expr,
            params: FnBodyParams::new(self.db, params),
        })
    }

    fn fn_param(&mut self, param: parsing::FnParam) -> Option<FnBodyParam<'db>> {
        if param.self_token().is_some() {
            return Some(FnBodyParam::new(self.db, FnBodyParamKind::SelfParam));
        }
        let pat = param.pattern().and_then(|p| self.pat(p));
        let expr = param.default_value().and_then(|e| self.expr(e));
        Some(FnBodyParam::new(
            self.db,
            FnBodyParamKind::Pat { pat, expr },
        ))
    }

    fn stmt(&mut self, stmt: parsing::Stmt) -> Option<Stmt<'db>> {
        Some(match stmt {
            parsing::Stmt::LetStmt(let_stmt) => {
                let expr = let_stmt.expr().and_then(|e| self.expr(e))?;
                let pat = let_stmt.pattern().and_then(|p| self.pat(p))?;
                let ty = let_stmt.ty().and_then(|ty| self.type_expr(ty));
                self.alloc_stmt(StmtKind::Let { pat, ty, expr }, stmt)
            }
            parsing::Stmt::ExprStmt(expr_stmt) => {
                let expr = self.expr(expr_stmt.expr()?)?;
                self.alloc_stmt(
                    StmtKind::Expr {
                        expr,
                        semi: expr_stmt.semi_token().map(|_| ()),
                    },
                    stmt,
                )
            }
        })
    }

    fn expr(&mut self, expr: parsing::Expr) -> Option<Expr<'db>> {
        Some(match expr {
            parsing::Expr::AsExpr(as_expr) => {
                let inner_expr = as_expr.expr().and_then(|e| self.expr(e))?;
                let ty = as_expr.type_expr().and_then(|ty| self.type_expr(ty))?;
                self.alloc_expr(
                    ExprKind::As {
                        expr: inner_expr,
                        ty,
                    },
                    expr,
                )
            }
            parsing::Expr::IsExpr(is_expr) => {
                let pat = is_expr.pat().and_then(|p| self.pat(p))?;
                let inner_expr = is_expr.expr().and_then(|e| self.expr(e))?;
                self.alloc_expr(
                    ExprKind::Is {
                        expr: inner_expr,
                        pat,
                    },
                    expr,
                )
            }
            parsing::Expr::IsNotExpr(is_not_expr) => {
                let pat = is_not_expr.pat().and_then(|p| self.pat(p))?;
                let inner_expr = is_not_expr.expr().and_then(|e| self.expr(e))?;
                self.alloc_expr(
                    ExprKind::IsNot {
                        expr: inner_expr,
                        pat,
                    },
                    expr,
                )
            }
            parsing::Expr::SelfExpr(_) => self.alloc_expr(ExprKind::SelfExpr, expr),
            parsing::Expr::ClosureExpr(closure_expr) => {
                let body = closure_expr.body().and_then(|e| self.expr(e))?;
                let output = closure_expr
                    .return_type()
                    .and_then(|o| o.ty())
                    .and_then(|ty| self.type_expr(ty));
                let params = closure_expr
                    .params()
                    .map(|p| p.params())
                    .into_iter()
                    .flatten()
                    .filter_map(|p| self.closure_param(p))
                    .collect_vec();
                self.alloc_expr(
                    ExprKind::Closure {
                        params: ClosureParams::new(self.db, params),
                        body,
                        output,
                    },
                    expr,
                )
            }
            parsing::Expr::FieldExpr(field_expr) => {
                let inner_expr = field_expr.expr().and_then(|e| self.expr(e))?;
                let name = field_expr.name().and_then(|n| n.text(self.source))?;
                self.alloc_expr(
                    ExprKind::Field {
                        name: Symbol::new(self.db, name),
                        expr: inner_expr,
                    },
                    expr,
                )
            }
            parsing::Expr::MethodExpr(method_expr) => {
                let inner_expr = method_expr.expr().and_then(|e| self.expr(e))?;
                let name = method_expr.name().and_then(|n| n.text(self.source))?;
                let generics = method_expr
                    .generic_args()
                    .map(|args| args.types())
                    .into_iter()
                    .flatten()
                    .map(|ty| self.type_expr(ty))
                    .collect_vec();
                let args = method_expr.args().filter_map(|a| self.arg(a)).collect_vec();
                self.alloc_expr(
                    ExprKind::Method {
                        expr: inner_expr,
                        name: Symbol::new(self.db, name),
                        generic_args: GenericArgs::new(self.db, generics),
                        args: Args::new(self.db, args),
                    },
                    expr,
                )
            }
            parsing::Expr::RecordExpr(record_expr) => {
                let path = record_expr.path().and_then(|p| self.path(p))?;
                let fields = record_expr
                    .fields_list()
                    .filter_map(|f| self.record_field(f))
                    .collect_vec();
                self.alloc_expr(
                    ExprKind::Record {
                        path,
                        fields: RecordFields::new(self.db, fields),
                    },
                    expr,
                )
            }
            parsing::Expr::UnitExpr(_) => self.alloc_expr(ExprKind::Unit, expr),
            parsing::Expr::PathExpr(path_expr) => {
                let path = path_expr.path().and_then(|p| self.path(p))?;
                self.alloc_expr(ExprKind::Path(path), expr)
            }
            parsing::Expr::BinaryExpr(binary_expr) => {
                let lhs = binary_expr.lhs().and_then(|e| self.expr(e))?;
                let rhs = binary_expr.rhs().and_then(|e| self.expr(e))?;
                let kind = binary_expr.op_kind()?;
                self.alloc_expr(ExprKind::Binary { lhs, rhs, kind }, expr)
            }
            parsing::Expr::UnaryExpr(unary_expr) => {
                let unary = unary_expr.expr().and_then(|e| self.expr(e))?;
                let kind = unary_expr.op_kind()?;
                self.alloc_expr(ExprKind::Unary { expr: unary, kind }, expr)
            }
            parsing::Expr::BlockExpr(block_expr) => self.block_expr(block_expr),
            parsing::Expr::IndexExpr(index_expr) => {
                let base = index_expr.base().and_then(|e| self.expr(e))?;
                let index = index_expr.index().and_then(|e| self.expr(e))?;
                self.alloc_expr(ExprKind::Index { base, index }, expr)
            }
            parsing::Expr::CallExpr(call_expr) => {
                let func = call_expr.func().and_then(|e| self.expr(e))?;
                let args = call_expr.args().filter_map(|a| self.arg(a)).collect_vec();
                self.alloc_expr(
                    ExprKind::Call {
                        func,
                        agrs: Args::new(self.db, args),
                    },
                    expr,
                )
            }
            parsing::Expr::ParenExpr(paren_expr) => {
                let inner_expr = paren_expr.expr().and_then(|e| self.expr(e))?;
                self.alloc_expr(ExprKind::Paren(inner_expr), expr)
            }
            parsing::Expr::ReturnExpr(return_expr) => {
                let inner_expr = return_expr.expr().and_then(|e| self.expr(e))?;
                self.alloc_expr(ExprKind::Return(inner_expr), expr)
            }
            parsing::Expr::LitExpr(lit_expr) => {
                let kind = lit_expr.kind()?;
                self.alloc_expr(ExprKind::Lit(kind), expr)
            }
            parsing::Expr::IfExpr(if_expr) => {
                let cond = if_expr.if_condition().and_then(|e| self.expr(e))?;
                let if_branch = if_expr.if_branch().map(|b| self.block_expr(b))?;
                let else_branch = if_expr.else_branch().map(|b| self.block_expr(b));
                self.alloc_expr(
                    ExprKind::If {
                        cond,
                        if_branch,
                        else_branch,
                    },
                    expr,
                )
            }
            parsing::Expr::TupleExpr(tuple_expr) => {
                let exprs = tuple_expr
                    .exprs()
                    .filter_map(|e| self.expr(e))
                    .collect_vec();
                self.alloc_expr(
                    ExprKind::Tuple {
                        exprs: ExprList::new(self.db, exprs),
                    },
                    expr,
                )
            }
        })
    }

    fn closure_param(&mut self, param: parsing::ClosureParam) -> Option<ClosureParam<'db>> {
        let pat = param.pattern().and_then(|p| self.pat(p))?;
        let ty = param.ty().and_then(|ty| self.type_expr(ty));
        Some(ClosureParam::new(self.db, pat, ty))
    }

    fn record_field(&mut self, field: parsing::RecordField) -> Option<RecordField<'db>> {
        let name = field.name().and_then(|n| n.text(self.source))?;
        let expr = field.expr().and_then(|e| self.expr(e))?;
        Some(RecordField::new(self.db, Symbol::new(self.db, name), expr))
    }

    fn arg(&mut self, arg: parsing::Arg) -> Option<Arg<'db>> {
        let value = arg.value().and_then(|e| self.expr(e))?;
        Some(
            if let Some(label) = arg.label().and_then(|n| n.text(self.source)) {
                Arg::new(
                    self.db,
                    ArgKind::Labeled {
                        label: Symbol::new(self.db, label),
                        value,
                    },
                )
            } else {
                Arg::new(self.db, ArgKind::NonLabeled { value })
            },
        )
    }

    fn block_expr(&mut self, block: parsing::BlockExpr) -> Expr<'db> {
        let stmts = block.stmts().filter_map(|s| self.stmt(s)).collect_vec();
        self.alloc_expr(
            ExprKind::Block {
                stmts: StmtList::new(self.db, stmts),
            },
            parsing::Expr::BlockExpr(block),
        )
    }

    fn pat(&mut self, pat: parsing::Pattern) -> Option<Pat<'db>> {
        Some(match pat {
            parsing::Pattern::NamePattern(name_pattern) => {
                let name = name_pattern
                    .name()
                    .and_then(|n| n.text(self.source))
                    .map(|name| Symbol::new(self.db, name))?;
                self.alloc_pat(PatKind::Name(name), pat)
            }
            parsing::Pattern::PathPattern(path_pattern) => {
                let path = path_pattern.path().and_then(|p| self.path(p))?;
                self.alloc_pat(PatKind::Path(path), pat)
            }
            parsing::Pattern::WildcardPattern(_) => self.alloc_pat(PatKind::Wildcard, pat),
        })
    }

    fn path(&mut self, path: parsing::Path) -> Option<Path<'db>> {
        let segments = path
            .segments()
            .filter_map(|s| self.path_segment(s))
            .collect_vec();
        if segments.is_empty() {
            return None;
        }
        Some(Path::new(self.db, segments))
    }

    fn path_segment(&mut self, path_segment: parsing::PathSegment) -> Option<PathSegment<'db>> {
        Some(PathSegment::new(
            self.db,
            Symbol::new(self.db, path_segment.ident(self.source)?),
            GenericArgs::new(
                self.db,
                path_segment
                    .generic_args()
                    .map(|args| args.types())
                    .into_iter()
                    .flatten()
                    .map(|ty| self.type_expr(ty))
                    .collect_vec(),
            ),
        ))
    }

    fn type_expr(&mut self, type_expr: parsing::TypeExpr) -> Option<TypeExpr<'db>> {
        Some(match type_expr {
            parsing::TypeExpr::DynType(dyn_type) => {
                let bounds = dyn_type
                    .bounds()
                    .filter_map(|b| self.type_expr(b))
                    .collect_vec();
                self.alloc_type_expr(
                    TypeExprKind::Dyn(TypeExprList::new(self.db, bounds)),
                    type_expr,
                )
            }
            parsing::TypeExpr::ParenType(paren_type) => {
                let ty = paren_type.type_expr().and_then(|ty| self.type_expr(ty))?;
                self.alloc_type_expr(TypeExprKind::Paren(ty), type_expr)
            }
            parsing::TypeExpr::PathType(path_type) => {
                let path = self.path(path_type.value()?)?;
                self.alloc_type_expr(TypeExprKind::Path(path), type_expr)
            }
            parsing::TypeExpr::NilableType(nilable_type) => {
                let ty = nilable_type
                    .ty()
                    .and_then(|nilable| self.type_expr(nilable))?;
                self.alloc_type_expr(TypeExprKind::Nilable(ty), type_expr)
            }
            parsing::TypeExpr::LitType(lit_type) => {
                self.alloc_type_expr(TypeExprKind::Lit(lit_type.kind()?), type_expr)
            }
            parsing::TypeExpr::FnType(fn_type) => {
                let output = fn_type
                    .output()
                    .and_then(|o| o.ty().and_then(|ty| self.type_expr(ty)));
                let params = fn_type
                    .param_list()
                    .map(|p| p.params())
                    .into_iter()
                    .flatten()
                    .map(|p| self.fn_type_param(p))
                    .collect_vec();
                self.alloc_type_expr(
                    TypeExprKind::Fn {
                        params: FnTypeParamList::new(self.db, params),
                        output,
                    },
                    type_expr,
                )
            }
            parsing::TypeExpr::AnyType(_) => self.alloc_type_expr(TypeExprKind::Any, type_expr),
            parsing::TypeExpr::UnitType(_) => self.alloc_type_expr(TypeExprKind::Unit, type_expr),
            parsing::TypeExpr::SelfType(_) => self.alloc_type_expr(TypeExprKind::SelfTy, type_expr),
            parsing::TypeExpr::TupleType(tuple_type) => {
                let types = tuple_type
                    .types()
                    .filter_map(|ty| self.type_expr(ty))
                    .collect_vec();
                self.alloc_type_expr(
                    TypeExprKind::Tuple(TypeExprList::new(self.db, types)),
                    type_expr,
                )
            }
        })
    }

    fn fn_type_param(&mut self, param: parsing::FnTypeParam) -> FnTypeParam<'db> {
        let name = param
            .name()
            .and_then(|n| n.text(self.source))
            .map(|n| Symbol::new(self.db, n));
        FnTypeParam::new(self.db, name, param.ty().and_then(|ty| self.type_expr(ty)))
    }

    fn generics(&mut self, generics: parsing::Generics) -> Option<Generics<'db>> {
        Some(Generics::new(
            self.db,
            generics
                .params()
                .filter_map(|p| self.generic_param(p))
                .collect_vec(),
        ))
    }

    fn generic_param(&mut self, param: parsing::TypeParam) -> Option<GenericParam<'db>> {
        Some(GenericParam::new(
            self.db,
            Symbol::new(self.db, param.name().and_then(|n| n.text(self.source))?),
            param
                .bounds()
                .filter_map(|b| self.type_expr(b))
                .collect_vec(),
        ))
    }

    fn alloc_type_expr(
        &mut self,
        type_expr: TypeExprKind<'db>,
        type_expr_ast: parsing::TypeExpr,
    ) -> TypeExpr<'db> {
        let id = self.map.insert_type_expr(type_expr_ast);
        TypeExpr::new(self.db, id, self.id_source.unwrap(), type_expr)
    }

    fn alloc_pat(&mut self, pat: PatKind<'db>, pat_ast: parsing::Pattern) -> Pat<'db> {
        let id = self.map.insert_pat(pat_ast);
        Pat::new(self.db, id, pat)
    }

    fn alloc_expr(&mut self, expr: ExprKind<'db>, expr_ast: parsing::Expr) -> Expr<'db> {
        let id = self.map.insert_expr(expr_ast);
        Expr::new(self.db, id, expr)
    }

    fn alloc_stmt(&mut self, stmt: StmtKind<'db>, stmt_ast: parsing::Stmt) -> Stmt<'db> {
        let id = self.map.insert_stmt(stmt_ast);
        Stmt::new(self.db, id, stmt)
    }
}

struct Ctx<'db, 'ast, 's> {
    db: &'db dyn salsa::Database,
    source: &'s str,
    ast_file: parsing::File<'ast>,
    file: File,
    ast_id_map: AstIdMap,
    items_map: ItemsMap<'db>,
}

impl<'db, 'ast, 's> Ctx<'db, 'ast, 's> {
    fn new(
        db: &'db dyn salsa::Database,
        source: &'s str,
        ast_file: parsing::File<'ast>,
        file: File,
    ) -> Self {
        Self {
            db,
            source,
            ast_file,
            file,
            ast_id_map: AstIdMap::new(),
            items_map: ItemsMap::default(),
        }
    }

    fn lower(mut self) -> (Vec<Item<'db>>, AstIdMap, ItemsMap<'db>) {
        let items = self.ast_file.items().collect_vec();
        (
            self.items(items.into_iter()),
            self.ast_id_map,
            self.items_map,
        )
    }

    fn items(&mut self, items_iter: impl Iterator<Item = parsing::Item<'ast>>) -> Vec<Item<'db>> {
        let mut items = vec![];
        for item in items_iter {
            match item {
                parsing::Item::FnItem(fn_item) => {
                    if let Some(fn_item) = self.fn_item(fn_item) {
                        items.push(Item::Function(fn_item));
                    }
                }
                parsing::Item::ModItem(mod_item) => {
                    if let Some(mod_item) = self.mod_item(mod_item) {
                        items.push(Item::Module(mod_item));
                    }
                }
                parsing::Item::ImplItem(impl_item) => {
                    if let Some(impl_item) = self.impl_item(impl_item) {
                        items.push(Item::Impl(impl_item));
                    }
                }
                parsing::Item::StructItem(struct_item) => {
                    if let Some(struct_item) = self.struct_item(struct_item) {
                        items.push(Item::Struct(struct_item));
                    }
                }
                parsing::Item::EnumItem(enum_item) => {
                    if let Some(enum_item) = self.enum_item(enum_item) {
                        items.push(Item::Enum(enum_item));
                    }
                }
                parsing::Item::UseItem(use_item) => {
                    if let Some(use_item) = self.use_item(use_item) {
                        items.push(Item::Use(use_item));
                    }
                }
            };
        }
        items
    }

    fn use_item(&mut self, use_item: parsing::UseItem<'ast>) -> Option<UseItem<'db>> {
        use_item.use_keyword()?;
        use_item.use_tree()?;
        let item = UseItem::new(
            self.db,
            self.file,
            UseItemId::new(self.ast_id_map.insert(use_item), self.file),
        );
        self.items_map.insert_use(self.db, item);
        Some(item)
    }

    fn enum_item(&mut self, enum_item: parsing::EnumItem<'ast>) -> Option<Enum<'db>> {
        enum_item.enum_token()?;
        let name = enum_item.name()?.text(self.source)?;
        let items = self.inner_items(enum_item.elements());

        let item = Enum::new(
            self.db,
            Symbol::new(self.db, name),
            self.file,
            items,
            EnumId::new(self.ast_id_map.insert(enum_item), self.file),
        );
        self.items_map.insert_enum(self.db, item);
        Some(item)
    }

    fn struct_item(&mut self, struct_item: parsing::StructItem<'ast>) -> Option<Struct<'db>> {
        struct_item.struct_token()?;
        let name = struct_item.name()?.text(self.source)?;
        let items = self.inner_items(struct_item.elements());

        let item = Struct::new(
            self.db,
            Symbol::new(self.db, name),
            self.file,
            items,
            StructId::new(self.ast_id_map.insert(struct_item), self.file),
        );
        self.items_map.insert_struct(self.db, item);
        Some(item)
    }

    fn inner_items(
        &mut self,
        elems: impl Iterator<Item = parsing::Elem<'ast>>,
    ) -> Vec<InnerItem<'db>> {
        let mut items = vec![];
        for elem in elems {
            match elem {
                parsing::Elem::Field(field) => {
                    let Some(ty) = field.ty() else {
                        continue;
                    };
                    match ty {
                        parsing::ItemTypeExpr::StructItem(struct_item)
                            if let Some(item) = self.struct_item(struct_item) =>
                        {
                            items.push(InnerItem::Struct(item));
                        }
                        parsing::ItemTypeExpr::EnumItem(enum_item)
                            if let Some(item) = self.enum_item(enum_item) =>
                        {
                            items.push(InnerItem::Enum(item));
                        }
                        _ => {}
                    }
                }
                parsing::Elem::FnItem(fn_item) => {
                    if let Some(item) = self.fn_item(fn_item) {
                        items.push(InnerItem::Function(item));
                    }
                }
            }
        }
        items
    }

    fn impl_item(&mut self, impl_item: parsing::ImplItem<'ast>) -> Option<ImplBlock<'db>> {
        let items = impl_item
            .functions()
            .filter_map(|item| self.fn_item(item))
            .collect_vec();
        let item = ImplBlock::new(
            self.db,
            self.file,
            ImplItems::new(self.db, items),
            ImplBlockId::new(self.ast_id_map.insert(impl_item), self.file),
        );
        self.items_map.insert_impl(self.db, item);
        Some(item)
    }

    fn fn_item(&mut self, fn_item: parsing::FnItem<'ast>) -> Option<Function<'db>> {
        let name = fn_item.name()?.text(self.source)?;
        let item = Function::new(
            self.db,
            Symbol::new(self.db, name),
            self.file,
            FunctionId::new(self.ast_id_map.insert(fn_item), self.file),
        );
        self.items_map.insert_fn(self.db, item);
        Some(item)
    }

    fn mod_item(&mut self, mod_item: parsing::ModItem<'ast>) -> Option<Module<'db>> {
        let id = self.ast_id_map.insert(mod_item);
        let name = mod_item.name().and_then(|n| n.text(self.source))?;
        let item = Module::new(
            self.db,
            Symbol::new(self.db, name),
            match mod_item.semi() {
                Some(_) => ModuleKind::Declaration {
                    id: ModuleId::new(id, self.file),
                },
                None => ModuleKind::Definition {
                    id: ModuleId::new(id, self.file),
                    items: Items::new(self.db, self.items(mod_item.items())),
                },
            },
            self.file.root(self.db),
        );
        self.items_map.insert_module(self.db, item);
        Some(item)
    }
}

struct ContentsMapCtx<'db, 's> {
    db: &'db dyn salsa::Database,
    source: &'s str,
    id_source: Option<IdSource<'db>>,
    map: ContentsMap,
    file: File,
}

impl<'db, 's> ContentsMapCtx<'db, 's> {
    fn new(db: &'db dyn salsa::Database, source: &'s str, file: File) -> Self {
        Self {
            db,
            file,
            source,
            map: ContentsMap::default(),
            id_source: None,
        }
    }

    fn impl_contents(mut self, item: ImplBlock<'db>) -> Option<ImplContents<'db>> {
        let ast_map = self.file.ast_map(self.db);
        let parse = self.file.parse(self.db);
        let ast_item = parse.cast::<parsing::ImplItem>(self.db, ast_map.get(item.id(self.db))?)?;

        self.id_source = Some(IdSource::ContentsSource(ContentsMapSource::Impl(item)));

        let first = ast_item.first_type().and_then(|ty| self.type_expr(ty));
        let second = ast_item.second_type().and_then(|ty| self.type_expr(ty));

        let impl_types = match (first, second) {
            (Some(inherent), None) => ImplTypes::Inherent(inherent),
            (Some(trait_ty), Some(impl_ty)) => ImplTypes::Trait { trait_ty, impl_ty },
            _ => return None,
        };
        let generics = ast_item
            .generics()
            .and_then(|g| self.generics(g))
            .unwrap_or_else(|| Generics::new(self.db, vec![]));

        Some(ImplContents {
            item_map: self.map.into(),
            generics,
            impl_types,
        })
    }

    fn fn_contents(mut self, item: Function<'db>) -> Option<FunctionContents<'db>> {
        let ast_map = self.file.ast_map(self.db);
        let parse = self.file.parse(self.db);
        let ast_item = parse.cast::<parsing::FnItem>(self.db, ast_map.get(item.id(self.db))?)?;

        self.id_source = Some(IdSource::ContentsSource(ContentsMapSource::Function(item)));

        let params = ast_item
            .params()
            .and_then(|p| self.fn_param_list(p))
            .unwrap_or_else(|| FnParamList::new(self.db, vec![]));
        let generics = ast_item
            .generics()
            .and_then(|g| self.generics(g))
            .unwrap_or_else(|| Generics::new(self.db, vec![]));
        let output = ast_item
            .output()
            .and_then(|o| o.ty())
            .and_then(|ty| self.type_expr(ty));

        Some(FunctionContents {
            item_map: self.map.into(),
            params,
            generics,
            output,
        })
    }

    //TODO: enums dont need whole body maps, if they wont *also* support fields. otherwise the
    //maximum they gonna have is a number: `MyValue = 0`
    fn enum_item(mut self, item: Enum<'db>) -> Option<EnumContents<'db>> {
        let ast_map = self.file.ast_map(self.db);
        let parse = self.file.parse(self.db);
        let ast_item = parse.cast::<parsing::EnumItem>(self.db, ast_map.get(item.id(self.db))?)?;

        self.id_source = Some(IdSource::ContentsSource(ContentsMapSource::Enum(item)));

        let elems = ast_item
            .elements()
            .filter_map(|e| self.elem(item.inner_items(self.db).iter().cloned(), e, None));

        let (elems, _): (Vec<_>, Vec<_>) = elems.unzip();

        Some(EnumContents {
            item_map: self.map.into(),
            elems: ElemList::new(self.db, elems),
        })
    }

    fn struct_item(mut self, item: Struct<'db>) -> Option<StructContents<'db>> {
        let ast_map = self.file.ast_map(self.db);
        let parse = self.file.parse(self.db);
        let ast_item =
            parse.cast::<parsing::StructItem>(self.db, ast_map.get(item.id(self.db))?)?;

        self.id_source = Some(IdSource::ContentsSource(ContentsMapSource::Struct(item)));

        let parent = ast_item
            .parent()
            .and_then(|p| p.path())
            .and_then(|p| self.path(p));
        let elems = ast_item.elements().filter_map(|e| {
            self.elem(
                item.inner_items(self.db).iter().cloned(),
                e,
                Some(item.id(self.db)),
            )
        });

        let (elems, bodies): (Vec<_>, Vec<_>) = elems.unzip();

        let field_bodies = bodies
            .into_iter()
            .zip(elems.iter())
            .filter_map(|(body, elem)| -> Option<_> {
                match elem.kind(self.db) {
                    ElemKind::Field(field) => body.map(|b| (field, Arc::new(b))),
                    ElemKind::Function(_) => None,
                }
            })
            .collect();

        Some(StructContents {
            item_map: self.map.into(),
            parent,
            elems: ElemList::new(self.db, elems),
            field_bodies,
        })
    }

    fn elem(
        &mut self,
        mut items: impl Iterator<Item = InnerItem<'db>>,
        elem: parsing::Elem,
        struct_id: Option<StructId>,
    ) -> Option<(Elem<'db>, Option<FieldBody<'db>>)> {
        Some(match elem {
            parsing::Elem::Field(field_ast) => {
                let name = field_ast
                    .name()
                    .and_then(|n| n.text(self.source))
                    .map(|n| Symbol::new(self.db, n));
                let ty = field_ast.ty().and_then(|ty| self.item_type_expr(items, ty));
                let field = Field::new(self.db, name, ty);
                let body = field_ast.default_value().and_then(|e| {
                    let mut ctx = BodyCtx::new(self.db, self.source, self.file);
                    ctx.id_source = Some(IdSource::BodySource(BodyMapSource::Field {
                        struct_id: struct_id.unwrap(),
                        field,
                    }));
                    let body_expr = ctx.expr(e)?;
                    Some(FieldBody {
                        body_map: ctx.map.into(),
                        body_expr,
                    })
                });
                (self.alloc_elem(ElemKind::Field(field), elem), body)
            }
            parsing::Elem::FnItem(fn_item) => {
                let name = Symbol::new(self.db, fn_item.name().and_then(|n| n.text(self.source))?);
                let item = items.find(|i| i.name(self.db) == name)?;
                let InnerItem::Function(item) = item else {
                    return None;
                };
                (self.alloc_elem(ElemKind::Function(item), elem), None)
            }
        })
    }

    fn item_type_expr(
        &mut self,
        mut items: impl Iterator<Item = InnerItem<'db>>,
        item_type_expr: parsing::ItemTypeExpr,
    ) -> Option<ItemTypeExpr<'db>> {
        Some(match item_type_expr {
            parsing::ItemTypeExpr::StructItem(struct_item) => {
                let name = Symbol::new(
                    self.db,
                    struct_item.name().and_then(|n| n.text(self.source))?,
                );
                let item = items.find(|i| i.name(self.db) == name)?;
                self.alloc_item_type_expr(
                    match item {
                        InnerItem::Struct(item) => ItemTypeExprKind::Struct(item),
                        InnerItem::Enum(item) => ItemTypeExprKind::Enum(item),
                        InnerItem::Function(_) => return None,
                    },
                    item_type_expr,
                )
            }
            parsing::ItemTypeExpr::EnumItem(enum_item) => {
                let name =
                    Symbol::new(self.db, enum_item.name().and_then(|n| n.text(self.source))?);
                let item = items.find(|i| i.name(self.db) == name)?;
                self.alloc_item_type_expr(
                    match item {
                        InnerItem::Struct(item) => ItemTypeExprKind::Struct(item),
                        InnerItem::Enum(item) => ItemTypeExprKind::Enum(item),
                        InnerItem::Function(_) => return None,
                    },
                    item_type_expr,
                )
            }
            parsing::ItemTypeExpr::TypeExpr(type_expr) => {
                let ty = self.type_expr(type_expr)?;
                self.alloc_item_type_expr(
                    ItemTypeExprKind::TypeExpr(ty),
                    parsing::ItemTypeExpr::TypeExpr(type_expr),
                )
            }
        })
    }

    fn fn_param_list(&mut self, param_list: parsing::ParamList) -> Option<FnParamList<'db>> {
        Some(FnParamList::new(
            self.db,
            param_list
                .params()
                .filter_map(|p| self.fn_param(p))
                .collect_vec(),
        ))
    }

    fn fn_param(&mut self, param: parsing::FnParam) -> Option<FnParam<'db>> {
        if param.self_token().is_some() {
            return Some(FnParam::new(self.db, FnParamKind::SelfParam));
        }
        let pat = param.pattern().and_then(|p| self.pat(p));
        let ty = param.type_expr().and_then(|ty| self.type_expr(ty));

        Some(FnParam::new(self.db, FnParamKind::Pat { pat, ty }))
    }

    fn type_expr(&mut self, type_expr: parsing::TypeExpr) -> Option<TypeExpr<'db>> {
        Some(match type_expr {
            parsing::TypeExpr::DynType(dyn_type) => {
                let bounds = dyn_type
                    .bounds()
                    .filter_map(|b| self.type_expr(b))
                    .collect_vec();
                self.alloc_type_expr(
                    TypeExprKind::Dyn(TypeExprList::new(self.db, bounds)),
                    type_expr,
                )
            }
            parsing::TypeExpr::ParenType(paren_type) => {
                let ty = paren_type.type_expr().and_then(|ty| self.type_expr(ty))?;
                self.alloc_type_expr(TypeExprKind::Paren(ty), type_expr)
            }
            parsing::TypeExpr::PathType(path_type) => {
                let path = self.path(path_type.value()?)?;
                self.alloc_type_expr(TypeExprKind::Path(path), type_expr)
            }
            parsing::TypeExpr::NilableType(nilable_type) => {
                let ty = nilable_type
                    .ty()
                    .and_then(|nilable| self.type_expr(nilable))?;
                self.alloc_type_expr(TypeExprKind::Nilable(ty), type_expr)
            }
            parsing::TypeExpr::LitType(lit_type) => {
                self.alloc_type_expr(TypeExprKind::Lit(lit_type.kind()?), type_expr)
            }
            parsing::TypeExpr::FnType(fn_type) => {
                let output = fn_type
                    .output()
                    .and_then(|o| o.ty().and_then(|ty| self.type_expr(ty)));
                let params = fn_type
                    .param_list()
                    .map(|p| p.params())
                    .into_iter()
                    .flatten()
                    .map(|p| self.fn_type_param(p))
                    .collect_vec();
                self.alloc_type_expr(
                    TypeExprKind::Fn {
                        params: FnTypeParamList::new(self.db, params),
                        output,
                    },
                    type_expr,
                )
            }
            parsing::TypeExpr::AnyType(_) => self.alloc_type_expr(TypeExprKind::Any, type_expr),
            parsing::TypeExpr::UnitType(_) => self.alloc_type_expr(TypeExprKind::Unit, type_expr),
            parsing::TypeExpr::SelfType(_) => self.alloc_type_expr(TypeExprKind::SelfTy, type_expr),
            parsing::TypeExpr::TupleType(tuple_type) => {
                let types = tuple_type
                    .types()
                    .filter_map(|ty| self.type_expr(ty))
                    .collect_vec();
                self.alloc_type_expr(
                    TypeExprKind::Tuple(TypeExprList::new(self.db, types)),
                    type_expr,
                )
            }
        })
    }

    fn fn_type_param(&mut self, param: parsing::FnTypeParam) -> FnTypeParam<'db> {
        let name = param
            .name()
            .and_then(|n| n.text(self.source))
            .map(|n| Symbol::new(self.db, n));
        FnTypeParam::new(self.db, name, param.ty().and_then(|ty| self.type_expr(ty)))
    }

    fn generics(&mut self, generics: parsing::Generics) -> Option<Generics<'db>> {
        Some(Generics::new(
            self.db,
            generics
                .params()
                .filter_map(|p| self.generic_param(p))
                .collect_vec(),
        ))
    }

    fn generic_param(&mut self, param: parsing::TypeParam) -> Option<GenericParam<'db>> {
        Some(GenericParam::new(
            self.db,
            Symbol::new(self.db, param.name().and_then(|n| n.text(self.source))?),
            param
                .bounds()
                .filter_map(|b| self.type_expr(b))
                .collect_vec(),
        ))
    }

    fn pat(&mut self, pat: parsing::Pattern) -> Option<Pat<'db>> {
        Some(match pat {
            parsing::Pattern::NamePattern(name_pattern) => {
                let name = name_pattern
                    .name()
                    .and_then(|n| n.text(self.source))
                    .map(|name| Symbol::new(self.db, name))?;
                self.alloc_pat(PatKind::Name(name), pat)
            }
            parsing::Pattern::PathPattern(path_pattern) => {
                let path = path_pattern.path().and_then(|p| self.path(p))?;
                self.alloc_pat(PatKind::Path(path), pat)
            }
            parsing::Pattern::WildcardPattern(_) => self.alloc_pat(PatKind::Wildcard, pat),
        })
    }

    fn alloc_elem(&mut self, elem: ElemKind<'db>, elem_ast: parsing::Elem) -> Elem<'db> {
        let id = self.map.insert_elem(elem_ast);
        Elem::new(self.db, id, elem)
    }

    fn alloc_item_type_expr(
        &mut self,
        type_expr: ItemTypeExprKind<'db>,
        type_expr_ast: parsing::ItemTypeExpr,
    ) -> ItemTypeExpr<'db> {
        let id = self.map.insert_item_type_expr(type_expr_ast);
        ItemTypeExpr::new(self.db, id, type_expr)
    }

    fn alloc_type_expr(
        &mut self,
        type_expr: TypeExprKind<'db>,
        type_expr_ast: parsing::TypeExpr,
    ) -> TypeExpr<'db> {
        let id = self.map.insert_type_expr(type_expr_ast);
        TypeExpr::new(self.db, id, self.id_source.unwrap(), type_expr)
    }

    fn alloc_pat(&mut self, pat: PatKind<'db>, pat_ast: parsing::Pattern) -> Pat<'db> {
        let id = self.map.insert_pat(pat_ast);
        Pat::new(self.db, id, pat)
    }

    fn path(&mut self, path: parsing::Path) -> Option<Path<'db>> {
        let segments = path
            .segments()
            .filter_map(|s| self.path_segment(s))
            .collect_vec();
        if segments.is_empty() {
            return None;
        }
        Some(Path::new(self.db, segments))
    }

    fn path_segment(&mut self, path_segment: parsing::PathSegment) -> Option<PathSegment<'db>> {
        Some(PathSegment::new(
            self.db,
            Symbol::new(self.db, path_segment.ident(self.source)?),
            GenericArgs::new(
                self.db,
                path_segment
                    .generic_args()
                    .map(|args| args.types())
                    .into_iter()
                    .flatten()
                    .map(|ty| self.type_expr(ty))
                    .collect_vec(),
            ),
        ))
    }
}

fn item_map_ctx<'db>(db: &'db dyn salsa::Database, file: File) -> ContentsMapCtx<'db, 'db> {
    let source = file.contents(db);
    ContentsMapCtx::new(db, source, file)
}

fn body_ctx<'db>(db: &'db dyn salsa::Database, file: File) -> BodyCtx<'db, 'db> {
    let source = file.contents(db);
    BodyCtx::new(db, source, file)
}

#[salsa::tracked]
impl<'db> ImplBlock<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn contents(self, db: &'db dyn salsa::Database) -> ImplContents<'db> {
        let ctx = item_map_ctx(db, self.file(db));
        ctx.impl_contents(self).unwrap()
    }
}

#[salsa::tracked]
impl<'db> Struct<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn contents(self, db: &'db dyn salsa::Database) -> StructContents<'db> {
        let ctx = item_map_ctx(db, self.file(db));
        ctx.struct_item(self).unwrap()
    }
}

#[salsa::tracked]
impl<'db> Enum<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn contents(self, db: &'db dyn salsa::Database) -> EnumContents<'db> {
        let ctx = item_map_ctx(db, self.file(db));
        ctx.enum_item(self).unwrap()
    }
}

#[salsa::tracked]
impl<'db> Function<'db> {
    #[salsa::tracked(returns(ref))]
    pub fn contents(self, db: &'db dyn salsa::Database) -> FunctionContents<'db> {
        let ctx = item_map_ctx(db, self.file(db));
        ctx.fn_contents(self).unwrap()
    }

    #[salsa::tracked(returns(ref))]
    pub fn body(self, db: &'db dyn salsa::Database) -> FunctionBody<'db> {
        let ctx = body_ctx(db, self.file(db));
        ctx.fn_item(self).unwrap()
    }
}

#[salsa::tracked]
impl<'db> UseItem<'db> {
    #[salsa::tracked(returns(clone))]
    pub fn use_tree_map(self, db: &'db dyn salsa::Database) -> Option<Arc<UseTreeMap>> {
        self.use_tree_and_map(db, self.file(db)).map(|(map, _)| map)
    }

    #[salsa::tracked(returns(clone))]
    pub fn use_tree(self, db: &'db dyn salsa::Database) -> Option<UseTree<'db>> {
        self.use_tree_and_map(db, self.file(db))
            .map(|(_, tree)| tree)
    }

    #[salsa::tracked(returns(clone))]
    pub fn use_tree_and_map(
        self,
        db: &'db dyn salsa::Database,
        file: File,
    ) -> Option<(Arc<UseTreeMap>, UseTree<'db>)> {
        fn use_tree_inner<'a, 'db>(
            db: &'db dyn salsa::Database,
            use_tree: parsing::UseTree<'a>,
            map: &mut UseTreeMap,
            source: &str,
        ) -> Option<UseTree<'db>> {
            let id = map.insert(use_tree);
            Some(UseTree::new(
                db,
                match use_tree {
                    parsing::UseTree::UseRootPath(root_path) => {
                        let use_tree = use_tree_inner(db, root_path.use_tree()?, map, source)?;
                        UseTreeKind::Root { use_tree }
                    }
                    parsing::UseTree::UseSuperPath(super_path) => {
                        let use_tree = use_tree_inner(db, super_path.use_tree()?, map, source)?;
                        UseTreeKind::Super { use_tree }
                    }
                    parsing::UseTree::UseSelf(_) => UseTreeKind::SelfUse,
                    parsing::UseTree::UseGlobal(_) => UseTreeKind::Global,
                    parsing::UseTree::UsePath(use_path) => {
                        let name = use_path
                            .name()
                            .and_then(|n| n.text(source))
                            .map(|t| Symbol::new(db, t))?;
                        let use_tree = use_tree_inner(db, use_path.use_tree()?, map, source)?;
                        UseTreeKind::Path { name, use_tree }
                    }
                    parsing::UseTree::UseName(use_name) => {
                        let name = use_name
                            .name()
                            .and_then(|n| n.text(source))
                            .map(|t| Symbol::new(db, t))?;
                        UseTreeKind::Name(name)
                    }
                    parsing::UseTree::UseTreeList(use_tree_list) => {
                        let use_trees = use_tree_list
                            .elements()
                            .filter_map(|tree| use_tree_inner(db, tree, map, source))
                            .collect_vec();
                        UseTreeKind::TreeList(UseTreeList::new(db, use_trees))
                    }
                },
                id,
            ))
        }
        let mut map = UseTreeMap::default();
        let parse = file.parse(db);
        let source = file.contents(db);

        let use_item = file.ast_map(db)[self.id(db)];
        let use_item = parse.cast::<parsing::UseItem>(db, use_item)?;
        let use_tree = use_item.use_tree()?;
        let use_tree = use_tree_inner(db, use_tree, &mut map, source)?;

        Some((Arc::new(map), use_tree))
    }
}

#[salsa::tracked]
impl<'db> File {
    #[salsa::tracked(returns(clone))]
    pub fn items(self, db: &'db dyn salsa::Database) -> Items<'db> {
        self.lower(db).0
    }

    #[salsa::tracked(returns(clone))]
    pub fn ast_map(self, db: &dyn salsa::Database) -> Arc<AstIdMap> {
        self.lower(db).1
    }

    #[salsa::tracked(returns(clone))]
    pub fn items_map(self, db: &'db dyn salsa::Database) -> Arc<ItemsMap<'db>> {
        self.lower(db).2
    }

    #[salsa::tracked(returns(clone))]
    fn lower(
        self,
        db: &'db dyn salsa::Database,
    ) -> (Items<'db>, Arc<AstIdMap>, Arc<ItemsMap<'db>>) {
        let parse = self.parse(db);
        let ctx = Ctx::new(db, self.contents(db), parse.file(db).unwrap(), self);
        let (items, id_map, items_map) = ctx.lower();
        let items = Items::new(db, items);
        (items.into(), id_map.into(), items_map.into())
    }
}
