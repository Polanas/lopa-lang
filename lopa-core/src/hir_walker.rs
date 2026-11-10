use crate::def::hir;

#[allow(unused_variables)]
pub trait HirWalker<'db> {
    fn item(&mut self, db: &'db dyn salsa::Database, item: hir::Item<'db>) {
        walk_item(db, self, item);
    }
    fn use_tree(&mut self, db: &'db dyn salsa::Database, use_tree: hir::UseTree<'db>) {
        walk_use_tree(db, self, use_tree);
    }

    fn expr(&mut self, db: &'db dyn salsa::Database, expr: hir::Expr<'db>) {
        walk_expr(db, self, expr);
    }
    fn stmt(&mut self, db: &'db dyn salsa::Database, stmt: hir::Stmt<'db>) {
        walk_stmt(db, self, stmt);
    }
    fn pat(&mut self, db: &'db dyn salsa::Database, pat: hir::Pat<'db>) {
        walk_pat(db, self, pat);
    }
    fn type_expr(&mut self, db: &'db dyn salsa::Database, type_expr: hir::TypeExpr<'db>) {
        walk_type_expr(db, self, type_expr);
    }
    fn item_type_expr(
        &mut self,
        db: &'db dyn salsa::Database,
        item_type_expr: hir::ItemTypeExpr<'db>,
    ) {
        walk_item_type_expr(db, self, item_type_expr);
    }
    fn member(&mut self, db: &'db dyn salsa::Database, member: hir::Member<'db>) {
        walk_member(db, self, member);
    }
}

pub fn walk_item<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    item: hir::Item<'db>,
) {
    match item {
        hir::Item::Function(item) => {
            let contents = item.contents(db);
            for param in contents.params.params(db).iter() {
                match param.kind(db) {
                    hir::FnParamKind::SelfParam => {}
                    hir::FnParamKind::Pat { ty, pat, expr } => {
                        if let Some(ty) = ty {
                            walk_type_expr(db, walker, ty);
                        }
                        if let Some(pat) = pat {
                            walk_pat(db, walker, pat);
                        }
                        if let Some(expr) = expr {
                            walk_expr(db, walker, expr);
                        }
                    }
                }
            }
            walk_expr(db, walker, contents.body_expr);
        }
        hir::Item::Struct(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walk_member(db, walker, *member);
            }
        }
        hir::Item::Class(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walk_member(db, walker, *member);
            }
        }
        hir::Item::Trait(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walk_member(db, walker, *member);
            }
        }
        hir::Item::Enum(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walk_member(db, walker, *member);
            }
        }
        hir::Item::Use(item) => {
            if let Some(use_tree) = item.use_tree(db) {
                walk_use_tree(db, walker, use_tree);
            }
        }
        hir::Item::Module(item) => {
            for item in item.items(db).items(db) {
                walk_item(db, walker, *item);
            }
        }
        hir::Item::Impl(item) => {
            for func in item.contents(db).fns.items(db).iter() {
                walk_item(db, walker, hir::Item::Function(*func));
            }
        }
    }
}
pub fn walk_use_tree<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    _walker: &mut W,
    use_tree: hir::UseTree<'db>,
) {
    match use_tree.kind(db) {
        hir::UseTreeKind::Path { use_tree, .. }
        | hir::UseTreeKind::Super { use_tree }
        | hir::UseTreeKind::Root { use_tree } => walk_use_tree(db, _walker, use_tree),
        hir::UseTreeKind::TreeList(use_tree_list) => {
            for use_tree in use_tree_list.items(db) {
                walk_use_tree(db, _walker, *use_tree);
            }
        }
        hir::UseTreeKind::Name(_) | hir::UseTreeKind::SelfUse | hir::UseTreeKind::Global => {}
    }
}

pub fn walk_member<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    member: hir::Member<'db>,
) {
    match member.kind(db) {
        hir::MemberKind::Field(field) => {
            if let Some(ty) = field.ty(db) {
                walk_item_type_expr(db, walker, ty);
            }
            if let Some(default) = field.default_value(db) {
                walk_expr(db, walker, default);
            }
        }
        hir::MemberKind::Function(function) => {
            walk_item(db, walker, hir::Item::Function(function));
        }
        hir::MemberKind::Variant(variant) => {
            if let Some(ty) = variant.ty(db) {
                walk_item_type_expr(db, walker, *ty);
            }
            if let Some(expr) = variant.expr(db) {
                walk_expr(db, walker, *expr);
            }
        }
    }
}

pub fn walk_item_type_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    item_type_expr: hir::ItemTypeExpr<'db>,
) {
    match item_type_expr.kind(db) {
        hir::ItemTypeExprKind::TypeExpr(type_expr) => {
            walk_type_expr(db, walker, type_expr);
        }
        hir::ItemTypeExprKind::Struct(item) => {
            walk_item(db, walker, hir::Item::Struct(item));
        }
        hir::ItemTypeExprKind::Enum(item) => {
            walk_item(db, walker, hir::Item::Enum(item));
        }
        hir::ItemTypeExprKind::Class(item) => {
            walk_item(db, walker, hir::Item::Class(item));
        }
    }
}

pub fn walk_type_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    _walker: &mut W,
    type_expr: hir::TypeExpr<'db>,
) {
    match type_expr.kind(db) {
        hir::TypeExprKind::Tuple(type_expr_list) => {
            for ty in type_expr_list.types(db) {
                walk_type_expr(db, _walker, *ty);
            }
        }
        hir::TypeExprKind::Dyn(type_expr_list) => {
            for ty in type_expr_list.types(db) {
                walk_type_expr(db, _walker, *ty);
            }
        }
        hir::TypeExprKind::Nilable(type_expr) | hir::TypeExprKind::Paren(type_expr) => {
            walk_type_expr(db, _walker, type_expr);
        }
        hir::TypeExprKind::Fn { params, output } => {
            for param in params.params(db).iter() {
                if let Some(ty) = param.ty(db) {
                    walk_type_expr(db, _walker, ty);
                }
            }
            if let Some(output) = output {
                walk_type_expr(db, _walker, output);
            }
        }
        hir::TypeExprKind::Any
        | hir::TypeExprKind::Unit
        | hir::TypeExprKind::Never
        | hir::TypeExprKind::Lit(_)
        | hir::TypeExprKind::Path(_)
        | hir::TypeExprKind::SelfTy => {}
    }
}

pub fn walk_pat<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    _walker: &mut W,
    pat: hir::Pat<'db>,
) {
    match pat.kind(db) {
        hir::PatKind::Paren(pat) => {
            walk_pat(db, _walker, pat);
        }
        hir::PatKind::Tuple(pat_list) => {
            for pat in pat_list.patterns(db).iter() {
                walk_pat(db, _walker, *pat);
            }
        }
        hir::PatKind::Wildcard | hir::PatKind::Name(_) | hir::PatKind::Path(_) => {}
    }
}
pub fn walk_stmt<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    stmt: hir::Stmt<'db>,
) {
    match stmt.kind(db) {
        hir::StmtKind::Let { pat, ty, expr } => {
            walk_expr(db, walker, expr);
            walk_pat(db, walker, pat);
            if let Some(ty) = ty {
                walk_type_expr(db, walker, ty);
            }
        }
        hir::StmtKind::Expr { expr, .. } => {
            walk_expr(db, walker, expr);
        }
    }
}

pub fn walk_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    expr: hir::Expr<'db>,
) {
    match expr.kind(db) {
        hir::ExprKind::As { expr, ty } => {
            walk_expr(db, walker, expr);
            walk_type_expr(db, walker, ty);
        }
        hir::ExprKind::Is { expr, pat } => {
            walk_expr(db, walker, expr);
            walk_pat(db, walker, pat);
        }
        hir::ExprKind::IsNot { expr, pat } => {
            walk_expr(db, walker, expr);
            walk_pat(db, walker, pat);
        }
        hir::ExprKind::Closure {
            params,
            body,
            output,
        } => {
            for param in params.params(db) {
                walk_pat(db, walker, param.pat(db));
                if let Some(ty) = param.ty(db) {
                    walk_type_expr(db, walker, ty);
                }
                walk_expr(db, walker, body);
                if let Some(output) = output {
                    walk_type_expr(db, walker, output);
                }
            }
        }
        hir::ExprKind::Field { expr, .. } => {
            walk_expr(db, walker, expr);
        }
        hir::ExprKind::Method { expr, args, .. } => {
            walk_expr(db, walker, expr);
            walk_args(db, walker, args);
        }
        hir::ExprKind::Record { fields, .. } => {
            for field in fields.fields(db).iter() {
                walk_expr(db, walker, field.expr(db));
            }
        }
        hir::ExprKind::Binary { lhs, rhs, .. } => {
            walk_expr(db, walker, lhs);
            walk_expr(db, walker, rhs);
        }
        hir::ExprKind::Unary { expr, .. } => {
            walk_expr(db, walker, expr);
        }
        hir::ExprKind::Index { base, index } => {
            walk_expr(db, walker, base);
            walk_expr(db, walker, index);
        }
        hir::ExprKind::Call { func, args } => {
            walk_expr(db, walker, func);
            walk_args(db, walker, args);
        }
        hir::ExprKind::Block { stmts } => {
            for stmt in stmts.stmts(db).iter() {
                walk_stmt(db, walker, *stmt);
            }
        }
        hir::ExprKind::Paren(expr) | hir::ExprKind::Return(expr) => {
            walk_expr(db, walker, expr);
        }
        hir::ExprKind::If(if_expr) => {
            walk_if_expr(db, walker, if_expr);
        }
        hir::ExprKind::Loop { block } => {
            walk_expr(db, walker, block);
        }
        hir::ExprKind::While { cond, block } => {
            walk_expr(db, walker, cond);
            walk_expr(db, walker, block);
        }
        hir::ExprKind::For {
            loop_expr,
            iterable,
            block,
        } => {
            walk_expr(db, walker, loop_expr);
            walk_expr(db, walker, iterable);
            walk_expr(db, walker, block);
        }
        hir::ExprKind::Tuple { exprs } => {
            for expr in exprs.exprs(db).iter() {
                walk_expr(db, walker, *expr);
            }
        }
        hir::ExprKind::Unit
        | hir::ExprKind::SelfExpr
        | hir::ExprKind::Lit(_)
        | hir::ExprKind::Path(_) => {}
    }
}

fn walk_args<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    args: hir::Args<'db>,
) {
    for arg in args.args(db).iter() {
        match arg.kind(db) {
            hir::ArgKind::Labeled { value, .. } | hir::ArgKind::NonLabeled { value } => {
                walk_expr(db, walker, value)
            }
        }
    }
}

pub fn walk_if_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    if_expr: hir::IfExpr<'db>,
) {
    walk_expr(db, walker, if_expr.cond(db));
    walk_expr(db, walker, if_expr.if_branch(db));
    if let Some(else_branch) = if_expr.else_branch(db) {
        match else_branch {
            hir::ElseBranch::Block(expr) => walk_expr(db, walker, expr),
            hir::ElseBranch::If(if_expr) => walk_if_expr(db, walker, if_expr),
        }
    }
}

// pub fn walk_pat<'db, W: HirWalker<'db> + ?Sized>(db: &'db dyn salsa::Database, walker: &mut W)
