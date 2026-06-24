use crate::parsing::lexer::Syntax::*;
use notify_rust::Notification;
use rowan::{SyntaxElementChildren, SyntaxNode, ast::AstNode};

use crate::{
    T, ide,
    parsing::{ast, lexer::Syntax, parser},
};

#[derive(Default)]
struct Context {
    output: String,
    ident_level: u32,
}

macro_rules! fmt {
    (@match $iter:ident, $self:ident, $(@next: $next:ident,)? $child:ident, $node:ident, $token:ident, { $($acc:tt)* }, $ast:ident($var:ident) if $cond:expr  => $b:block $($tail:tt)* ) => {
        fmt! {
            @match
            $iter,
            $self,
            $(@next: $next,)?
            $child,
            $node,
            $token,
            {
                $($acc)*
                $var if $cond => {
                    let node = $child.as_node().cloned().unwrap();
                    if let Some($node) = ast::$ast::cast(node) {
                        $(
                            #[allow(unused_variables)]
                            let $next = $iter.peek();
                        )?
                        $b
                    }
                },
            },
            $($tail)*
        }

    };
    (@match $iter:ident, $self:ident, $(@next: $next:ident,)? $child:ident, $node:ident, $token:ident, { $($acc:tt)* }, $ast:ident($kind:ident) => $b:block $($tail:tt)* ) => {
        fmt! {
            @match
            $iter,
            $self,
            $(@next: $next,)?
            $child,
            $node,
            $token,
            {
                $($acc)*
                $kind => {
                    #[allow(unused_variables)]
                    let node = $child.as_node().cloned().unwrap();
                    if let Some($node) = ast::$ast::cast(node) {
                        $(
                            #[allow(unused_variables)]
                            let $next = $iter.peek();
                        )?
                        $b
                    }
                },
            },
            $($tail)*
        }

    };
    (@match $iter:ident, $self:ident, $(@next: $next:ident,)? $child:ident, $node:ident, $token:ident, { $($acc:tt)* }, $kind:ident => $b:block $($tail:tt)* ) => {
        fmt! {
            @match
            $iter,
            $self,
            $(@next: $next,)?
            $child,
            $node,
            $token,
            {
                $($acc)*
                $kind => {
                    #[allow(unused_variables)]
                    let $token = $child.as_token().unwrap();
                    $(
                        #[allow(unused_variables)]
                        let $next = $iter.peek();
                    )?
                    $b
                },
            },
            $($tail)*
        }
    };
    (@match $iter:ident, $self:ident, $(@next: $next:ident,)? $child:ident, $node:ident, $token:ident, { $($acc:tt)* }, T![$tok:tt] => $b:block $($tail:tt)* ) => {
        fmt! {
            @match
            $iter,
            $self,
            $(@next: $next,)?
            $child,
            $node,
            $token,
            {
                $($acc)*
                T![$tok] => {
                    #[allow(unused_variables)]
                    let $token = $child.as_token().unwrap();
                    $(
                        #[allow(unused_variables)]
                        let $next = $iter.peek();
                    )?
                    $b
                },
            },
            $($tail)*
        }
    };
    (@match $iter:ident, $self:ident, $(@next: $next:ident,)? $child:ident, $node:ident, $token:ident, { $($acc:tt)* },) => {
        match $child.kind() {
            $($acc)*
            ERROR => {
                #[allow(unused_variables)]
                let token = $child.as_token().unwrap();
                $self.token(token);
                if let Some(rowan::NodeOrToken::Token(token)) = $iter.peek()
                    && token.kind() == WHITESPACE
                {
                    $self.token(token);
                }
            }
            _ => {}
        }
    };
    ($ast_item:ident, $self:ident, $($next:ident,)? |$node:ident, $token:ident| $($input:tt)*) => {
        let mut iter = $ast_item.syntax().children_with_tokens().peekable();
        while let Some(child) = iter.next() {
            fmt! {
                @match
                iter,
                $self,
                $(@next: $next,)?
                child,
                $node,
                $token,
                { },
                $($input)*
            }
        }
    };
}

impl Context {
    fn format(&mut self, file: ast::File) {
        fmt! {
            file, self, |node, token|
            FnItem(FN_ITEM) => {
                self.fn_item(node);
            }
        }
    }

    fn fn_item(&mut self, fn_item: ast::FnItem) {
        fmt! {
            fn_item, self, |node, token|
            T![fn] => {
                self.token_space(token);
            }
            Name(NAME) => {
                self.name(node);
            }
            ParamList(PARAM_LIST) => {
                self.param_list(node);
            }
            ReturnType(RETURN_TYPE) => {
                self.output(node);
            }
            BlockExpr(BLOCK_EXPR) => {
                self.expr(ast::Expr::BlockExpr(node));
            }
        }
    }

    fn output(&mut self, output: ast::ReturnType) {
        fmt! {
            output, self, |node, token|
            T![->] => {
                self.token(token);
            }
            TypeExpr(expr) if expr.is_type_expr() => {
                self.type_expr(node);
            }
        }
    }

    fn param_list(&mut self, param_list: ast::ParamList) {
        fmt! {
            param_list, self, next, |node, token|
            T!["("] => {
                self.token(token);
            }
            T![" "] => {
                if let Some(rowan::NodeOrToken::Token(token)) = next
                    && token.kind() == T![")"] {
                    self.output.remove(self.output.len()-1);
                }

            }
            FnParam(PARAM) => {
                let has_comma = node.comma_token().is_some();
                self.param(node);
                if let Some(rowan::NodeOrToken::Token(token)) = next
                    && token.kind() == T![")"]
                    && has_comma
                    && self.output.ends_with(' ') {
                    self.output.remove(self.output.len()-1);
                }
            }
            T![")"] => {
                self.token_space(token);
            }
        }
    }

    fn param(&mut self, param: ast::FnParam) {
        fmt! {
            param, self, next, |node, token|
            T![self] => {
                self.token(token);
            }
            Pattern(pat) if pat.is_pattern() => {
                self.pattern(node);
            }
            T![:] => {
                self.token_space(token);
            }
            TypeExpr(expr) if expr.is_type_expr() => {
                self.type_expr(node);
            }
            T![=] => {
                self.token(token);
            }
            Expr(expr) if expr.is_expr() => {
                self.expr(node);
            }
            T![,] => {
                self.token_space(token);
            }
        }
    }

    fn expr(&mut self, expr: ast::Expr) {
        match expr {
            ast::Expr::AsExpr(as_expr) => {}
            ast::Expr::IsExpr(is_expr) => {}
            ast::Expr::IsNotExpr(is_not_expr) => {}
            ast::Expr::SelfExpr(self_expr) => {}
            ast::Expr::ClosureExpr(closure_expr) => {}
            ast::Expr::FieldExpr(field_expr) => {}
            ast::Expr::MethodExpr(method_expr) => {}
            ast::Expr::RecordExpr(record_expr) => {}
            ast::Expr::UnitExpr(unit_expr) => {}
            ast::Expr::PathExpr(path_expr) => {
                self.path_expr(path_expr);
            }
            ast::Expr::BinaryExpr(binary_expr) => {}
            ast::Expr::UnaryExpr(unary_expr) => {}
            ast::Expr::BlockExpr(block_expr) => {
                self.block_expr(block_expr);
            }
            ast::Expr::IndexExpr(index_expr) => {}
            ast::Expr::CallExpr(call_expr) => {}
            ast::Expr::ParenExpr(paren_expr) => {}
            ast::Expr::ReturnExpr(return_expr) => {}
            ast::Expr::LitExpr(lit_expr) => {}
            ast::Expr::TryExpr(try_expr) => {}
            ast::Expr::IfExpr(if_expr) => {}
        }
    }

    fn path_expr(&mut self, path_expr: ast::PathExpr) {
        fmt! {
            path_expr, self, |node, token|
            Path(PATH) => {
                self.path(node);
            }
        }
    }

    fn block_expr(&mut self, block_expr: ast::BlockExpr) {
        fmt! {
            block_expr, self, |node, token|
            T!["{"] => {
                self.token(token);
                self.new_line();
                self.acc_ident();
            }
            T!["}"] => {
                self.dec_ident();
                self.new_line();
                self.token(token);
            }
        }
    }

    fn type_expr(&mut self, type_expr: ast::TypeExpr) {
        match type_expr {
            ast::TypeExpr::DynType(dyn_type) => {}
            ast::TypeExpr::PathType(path_type) => {
                self.path_type(path_type);
            }
            ast::TypeExpr::NilableType(nilable_type) => {}
            ast::TypeExpr::LitType(lit_type) => {
                if let Some(path) = lit_type.path() {
                    self.path_type(path);
                }
            }
            ast::TypeExpr::AnyType(any_type) => {
                if let Some(path) = any_type.path() {
                    self.path_type(path);
                }
            }
            ast::TypeExpr::UnitType(unit_type) => {
                if let Some(path) = unit_type.path() {
                    self.path_type(path);
                }
            }
            ast::TypeExpr::FnType(fn_type) => {}
            ast::TypeExpr::SelfType(self_type) => {}
        }
    }

    fn path_type(&mut self, path: ast::PathType) {
        fmt! {
            path, self, |node, token|
            Path(PATH) => {
                self.path(node);
            }
        }
    }

    fn path(&mut self, path: ast::Path) {
        fmt! {
            path, self, |node, token|
            T![:] => {
                self.token(token);
            }
            PathSegment(PATH_SEGMENT) => {
                self.path_segment(node);
            }
        }
    }

    fn path_segment(&mut self, segment: ast::PathSegment) {
        //TODO:generics
        fmt! {
            segment, self, |node, token|
        }
        if let Some(ident) = segment.ident() {
            self.text(&ident);
        }
    }

    fn pattern(&mut self, pattern: ast::Pattern) {
        match pattern {
            ast::Pattern::NamePattern(pattern) => {
                fmt! {
                    pattern, self, |node, token|
                    Name(NAME) => {
                        self.name(node);
                    }
                }
            }
            ast::Pattern::PathPattern(path_pattern) => {}
            ast::Pattern::WildcardPattern(wildcard_pattern) => {}
        }
    }

    fn name(&mut self, name: ast::Name) {
        fmt! {
            name, self, |node, token|
        }
        self.text_opt(name.text().as_deref());
    }

    fn with_acc_ident(&mut self, f: impl FnOnce(&mut Self)) {
        self.acc_ident();
        f(self);
        self.dec_ident();
    }

    fn dec_ident(&mut self) {
        if self.ident_level == 0 {
            return;
        }
        self.ident_level -= 1;
    }

    fn acc_ident(&mut self) {
        self.ident_level += 1;
    }

    fn token_space_opt(&mut self, token: Option<&ast::SyntaxToken>) {
        if let Some(token) = token {
            self.token_space(token);
        }
    }

    fn token_space(&mut self, token: &ast::SyntaxToken) {
        self.token(token);
        self.space();
    }

    fn token_opt(&mut self, token: Option<&ast::SyntaxToken>) {
        if let Some(token) = token {
            self.token(token);
        }
    }

    fn token(&mut self, token: &ast::SyntaxToken) {
        self.text(token.text());
    }

    fn text_space_opt(&mut self, text: Option<&str>) {
        if let Some(text) = text {
            self.text_space(text);
        }
    }

    fn text_space(&mut self, text: &str) {
        self.text(text);
        self.space();
    }

    fn text_opt(&mut self, text: Option<&str>) {
        if let Some(text) = text {
            self.text(text);
        }
    }

    fn text(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn space(&mut self) {
        self.output.push(' ');
    }

    fn new_line(&mut self) {
        self.output.push('\n');
        for _ in 0..self.ident_level {
            self.output.push_str("  ");
        }
    }
}

#[salsa::tracked]
pub fn format_file(db: &dyn salsa::Database, file: ide::File) -> String {
    let mut ctx = Context::default();
    ctx.format(ide::parse(db, file).file(db));
    ctx.output
}
