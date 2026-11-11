use notify_rust::Notification;

use crate::def::hir;

#[allow(unused_variables)]
pub trait HirWalker<'db> {
    fn walk_item(&mut self, db: &'db dyn salsa::Database, item: hir::Item<'db>) {
        walk_item(db, self, item);
    }
    fn walk_use_tree(&mut self, db: &'db dyn salsa::Database, use_tree: hir::UseTree<'db>) {
        walk_use_tree(db, self, use_tree);
    }

    fn walk_expr(&mut self, db: &'db dyn salsa::Database, expr: hir::Expr<'db>) {
        walk_expr(db, self, expr);
    }
    fn walk_stmt(&mut self, db: &'db dyn salsa::Database, stmt: hir::Stmt<'db>) {
        walk_stmt(db, self, stmt);
    }
    fn walk_pat(&mut self, db: &'db dyn salsa::Database, pat: hir::Pat<'db>) {
        walk_pat(db, self, pat);
    }
    fn walk_type_expr(&mut self, db: &'db dyn salsa::Database, type_expr: hir::TypeExpr<'db>) {
        walk_type_expr(db, self, type_expr);
    }
    fn walk_item_type_expr(
        &mut self,
        db: &'db dyn salsa::Database,
        item_type_expr: hir::ItemTypeExpr<'db>,
    ) {
        walk_item_type_expr(db, self, item_type_expr);
    }
    fn walk_member(&mut self, db: &'db dyn salsa::Database, member: hir::Member<'db>) {
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
                            walker.walk_type_expr(db, ty);
                        }
                        if let Some(pat) = pat {
                            walker.walk_pat(db, pat);
                        }
                        if let Some(expr) = expr {
                            walker.walk_expr(db, expr);
                        }
                    }
                }
            }
            if let Some(body_expr) = contents.body_expr {
                walker.walk_expr(db, body_expr);
            }
        }
        hir::Item::Struct(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walker.walk_member(db, *member);
            }
        }
        hir::Item::Class(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walker.walk_member(db, *member);
            }
        }
        hir::Item::Trait(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walker.walk_member(db, *member);
            }
        }
        hir::Item::Enum(item) => {
            let contents = item.contents(db);
            for member in contents.members.members(db).iter() {
                walker.walk_member(db, *member);
            }
        }
        hir::Item::Use(item) => {
            if let Some(use_tree) = item.use_tree(db) {
                walker.walk_use_tree(db, use_tree);
            }
        }
        hir::Item::Module(item) => {
            for item in item.items(db).items(db) {
                walker.walk_item(db, *item);
            }
        }
        hir::Item::Impl(item) => {
            for func in item.contents(db).fns.items(db).iter() {
                walker.walk_item(db, hir::Item::Function(*func));
            }
        }
    }
}
pub fn walk_use_tree<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    use_tree: hir::UseTree<'db>,
) {
    match use_tree.kind(db) {
        hir::UseTreeKind::Path { use_tree, .. }
        | hir::UseTreeKind::Super { use_tree }
        | hir::UseTreeKind::Root { use_tree } => walker.walk_use_tree(db, use_tree),
        hir::UseTreeKind::TreeList(use_tree_list) => {
            for use_tree in use_tree_list.items(db) {
                walker.walk_use_tree(db, *use_tree);
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
                walker.walk_item_type_expr(db, ty);
            }
            if let Some(default) = field.default_value(db) {
                walker.walk_expr(db, default);
            }
        }
        hir::MemberKind::Function(function) => {
            walker.walk_item(db, hir::Item::Function(function));
        }
        hir::MemberKind::Variant(variant) => {
            if let Some(ty) = variant.ty(db) {
                walker.walk_item_type_expr(db, *ty);
            }
            if let Some(expr) = variant.expr(db) {
                walker.walk_expr(db, *expr);
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
            walker.walk_type_expr(db, type_expr);
        }
        hir::ItemTypeExprKind::Struct(item) => {
            walker.walk_item(db, hir::Item::Struct(item));
        }
        hir::ItemTypeExprKind::Enum(item) => {
            walker.walk_item(db, hir::Item::Enum(item));
        }
        hir::ItemTypeExprKind::Class(item) => {
            walker.walk_item(db, hir::Item::Class(item));
        }
    }
}

pub fn walk_type_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    type_expr: hir::TypeExpr<'db>,
) {
    match type_expr.kind(db) {
        hir::TypeExprKind::Tuple(type_expr_list) | hir::TypeExprKind::Dyn(type_expr_list) => {
            for ty in type_expr_list.types(db) {
                walker.walk_type_expr(db, *ty);
            }
        }
        hir::TypeExprKind::Nilable(type_expr) | hir::TypeExprKind::Paren(type_expr) => {
            walker.walk_type_expr(db, type_expr);
        }
        hir::TypeExprKind::Fn { params, output } => {
            for param in params.params(db).iter() {
                if let Some(ty) = param.ty(db) {
                    walker.walk_type_expr(db, ty);
                }
            }
            if let Some(output) = output {
                walker.walk_type_expr(db, output);
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
    walker: &mut W,
    pat: hir::Pat<'db>,
) {
    match pat.kind(db) {
        hir::PatKind::Paren(pat) => {
            walker.walk_pat(db, pat);
        }
        hir::PatKind::Tuple(pat_list) => {
            for pat in pat_list.patterns(db).iter() {
                walker.walk_pat(db, *pat);
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
            walker.walk_expr(db, expr);
            walker.walk_pat(db, pat);
            if let Some(ty) = ty {
                walker.walk_type_expr(db, ty);
            }
        }
        hir::StmtKind::Expr { expr, .. } => {
            walker.walk_expr(db, expr);
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
            walker.walk_expr(db, expr);
            walker.walk_type_expr(db, ty);
        }
        hir::ExprKind::Is { expr, pat } => {
            walker.walk_expr(db, expr);
            walker.walk_pat(db, pat);
        }
        hir::ExprKind::IsNot { expr, pat } => {
            walker.walk_expr(db, expr);
            walker.walk_pat(db, pat);
        }
        hir::ExprKind::Closure {
            params,
            body,
            output,
        } => {
            for param in params.params(db) {
                walker.walk_pat(db, param.pat(db));
                if let Some(ty) = param.ty(db) {
                    walker.walk_type_expr(db, ty);
                }
                walker.walk_expr(db, body);
            }
            if let Some(output) = output {
                walker.walk_type_expr(db, output);
            }
        }
        hir::ExprKind::Field { expr, .. } => {
            walker.walk_expr(db, expr);
        }
        hir::ExprKind::Method { expr, args, .. } => {
            walker.walk_expr(db, expr);
            walk_args(db, walker, args);
        }
        hir::ExprKind::Record { fields, .. } => {
            for field in fields.fields(db).iter() {
                walker.walk_expr(db, field.expr(db));
            }
        }
        hir::ExprKind::Binary { lhs, rhs, .. } => {
            walker.walk_expr(db, lhs);
            walker.walk_expr(db, rhs);
        }
        hir::ExprKind::Unary { expr, .. } => {
            walker.walk_expr(db, expr);
        }
        hir::ExprKind::Index { base, index } => {
            walker.walk_expr(db, base);
            walker.walk_expr(db, index);
        }
        hir::ExprKind::Call { func, args } => {
            walker.walk_expr(db, func);
            walk_args(db, walker, args);
        }
        hir::ExprKind::Block { stmts } => {
            for stmt in stmts.stmts(db).iter() {
                walker.walk_stmt(db, *stmt);
            }
        }
        hir::ExprKind::Paren(expr) | hir::ExprKind::Return(expr) => {
            walker.walk_expr(db, expr);
        }
        hir::ExprKind::If(if_expr) => {
            walk_if_expr(db, walker, if_expr);
        }
        hir::ExprKind::Loop { block } => {
            walker.walk_expr(db, block);
        }
        hir::ExprKind::While { cond, block } => {
            walker.walk_expr(db, cond);
            walker.walk_expr(db, block);
        }
        hir::ExprKind::For {
            loop_expr,
            iterable,
            block,
        } => {
            walker.walk_expr(db, loop_expr);
            walker.walk_expr(db, iterable);
            walker.walk_expr(db, block);
        }
        hir::ExprKind::Tuple { exprs } => {
            for expr in exprs.exprs(db).iter() {
                walker.walk_expr(db, *expr);
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
                walker.walk_expr(db, value);
            }
        }
    }
}

pub fn walk_if_expr<'db, W: HirWalker<'db> + ?Sized>(
    db: &'db dyn salsa::Database,
    walker: &mut W,
    if_expr: hir::IfExpr<'db>,
) {
    walker.walk_expr(db, if_expr.cond(db));
    walker.walk_expr(db, if_expr.if_branch(db));
    if let Some(else_branch) = if_expr.else_branch(db) {
        match else_branch {
            hir::ElseBranch::Block(expr) => walker.walk_expr(db, expr),
            hir::ElseBranch::If(if_expr) => walk_if_expr(db, walker, if_expr),
        }
    }
}
