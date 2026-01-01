use crate::{ast, common, position::WithSpan};

pub struct Context {
    result: String,
    stack: Vec<usize>,
}

impl Context {
    fn new() -> Self {
        Self {
            result: Default::default(),
            stack: vec![0],
        }
    }

    fn stack_mut(&mut self) -> &mut usize {
        let len = self.stack.len();
        &mut self.stack[len - 1]
    }

    fn stack(&self) -> usize {
        self.stack[self.stack.len() - 1]
    }

    fn push(&mut self) -> usize {
        let stack = self.stack();
        *self.stack_mut() += 1;
        stack
    }

    fn push_stack(&mut self) {
        let stack = self.stack();
        self.stack.push(stack);
    }

    fn pop_stack(&mut self) {
        self.stack.pop();
    }

    fn ident(&self, id: usize) -> String {
        format!("stack_{id}")
    }

    fn clear_stack(&mut self) {
        self.stack = vec![0];
    }

    fn push_ident(&mut self) -> String {
        let push = self.push();
        self.ident(push)
    }

    fn pop(&mut self) -> usize {
        *self.stack_mut() -= 1;
        self.stack()
    }

    fn pop_ident(&mut self) -> String {
        let pop = self.pop();
        self.ident(pop)
    }

    fn generate(&mut self, program: &[WithSpan<ast::Stmt>]) {
        match program {
            [item] => {
                let stmt = self.stmt(&item.value);
                self.clear_stack();
                self.result.push_str(&stmt);
            }
            [items @ .., last] => {
                for item in items {
                    let stmt = self.stmt(&item.value);
                    self.clear_stack();
                    self.result.push_str(&format!("{stmt}\n"));
                }
                let stmt = self.stmt(&last.value);
                self.result.push_str(&stmt);
            }
            [] => {}
        }
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> String {
        let mut result = String::new();
        match &stmt {
            ast::Stmt::Expr(stmt_expr) => todo!(),
            ast::Stmt::Item(item) => todo!(),
            ast::Stmt::Assign(idents, values) => todo!(),
            ast::Stmt::Binding(binding) => {
                if let Some(values) = &binding.values {
                    if binding.identifiers.len() == values.len() {
                        result.push_str("local ");
                        match &binding.identifiers.as_slice() {
                            [item] => {
                                result.push_str(&item.value);
                            }
                            [items @ .., last] => {
                                for item in items {
                                    result.push_str(&format!("{},", &item.value));
                                }
                                result.push_str(&last.value);
                            }
                            [] => unreachable!(),
                        };
                        result.push_str(" = ");
                        match &values.as_slice() {
                            [item] => {
                                if let Some(expr) = &self.expr(&item.value) {
                                    result.push_str(expr);
                                }
                            }
                            [items @ .., last] => {
                                for item in items {
                                    if let Some(expr) = &self.expr(&item.value) {
                                        result.push_str(&format!("{expr},"));
                                    }
                                }
                                if let Some(expr) = &self.expr(&last.value) {
                                    result.push_str(expr);
                                }
                            }
                            [] => unreachable!(),
                        };
                    } else {
                        let mut stack = self.stack();
                        for item in values {
                            match &item.value {
                                item @ ast::Expr::Block(_) => {
                                    self.expr(item);
                                }
                                _ => {
                                    if let Some(expr) = self.expr(&item.value) {
                                        result.push_str(&format!(
                                            "{} = {}\n",
                                            self.push_ident(),
                                            expr
                                        ));
                                    }
                                }
                            }
                            // result.push_str(&format!(
                            //     "{} = {}\n",
                            //     self.push_ident(),
                            //     self.expr(&item.value)
                            // ));
                        }
                        for ident in binding.identifiers.iter() {
                            result.push_str(&format!(
                                "local {} = {}\n",
                                &ident.value,
                                self.ident(stack)
                            ));
                            stack += 1;
                        }
                    }
                } else {
                    result.push_str("local ");
                    match &binding.identifiers.as_slice() {
                        [item] => {
                            result.push_str(&item.value);
                        }
                        [items @ .., last] => {
                            for item in items {
                                result.push_str(&format!("{},", &item.value));
                            }
                            result.push_str(&last.value);
                        }
                        [] => unreachable!(),
                    };
                }
            }
            ast::Stmt::Print(e) => {
                if let Some(expr) = self.expr(&e.value) {
                    self.result.push_str(&format!("print({expr})"));
                }
            }
            ast::Stmt::Empty => todo!(),
        };
        result
    }

    fn expr(&mut self, expr: &ast::Expr) -> Option<String> {
        match &expr {
            ast::Expr::Nil => Some(String::from("nil")),
            ast::Expr::Number(number) => match number {
                ast::Number::Float(f) => Some(f.to_string()),
                ast::Number::Int(i) => Some(i.to_string()),
            },
            ast::Expr::Bool(b) => Some(b.to_string()),
            ast::Expr::String(s) => Some(format!("\"{s}\"")),
            ast::Expr::Grouping(e) => self.expr(&e.value),
            ast::Expr::Unary(op, e) => match op.value {
                common::UnaryOp::Not => self.expr(&e.value).map(|e| format!("not ({})", e)),
                common::UnaryOp::Negate => self.expr(&e.value).map(|e| format!("-{}", e)),
            },
            ast::Expr::Binary(binary_expr) => {
                let op = match binary_expr.op.value {
                    common::BinaryOp::Div => "/",
                    common::BinaryOp::Mult => "*",
                    common::BinaryOp::Add => "+",
                    common::BinaryOp::Sub => "-",
                    common::BinaryOp::Greater => ">",
                    common::BinaryOp::GreaterEqual => ">=",
                    common::BinaryOp::Less => "<",
                    common::BinaryOp::LessEqual => "<=",
                    common::BinaryOp::NotEqual => "~=",
                    common::BinaryOp::Equal => "==",
                    common::BinaryOp::Modulo => "%",
                    common::BinaryOp::And => " and ",
                    common::BinaryOp::Or => " or ",
                };
                if let (Some(l), Some(r)) = (
                    self.expr(&binary_expr.left.value),
                    self.expr(&binary_expr.right.value),
                ) {
                    Some(format!("{}{}{}", l, op, r))
                } else {
                    None
                }
            }
            ast::Expr::Identifier(i) => Some(i.to_string()),
            ast::Expr::Call(_, items) => todo!(),
            ast::Expr::If(if_expr) => {
                todo!()
                // let condition = self.expr(&if_expr.condition.value).unwrap();
                // let mut result = String::new();
                // result.push_str(&format!("if {} then", &condition));
                // result
            }
            ast::Expr::Block(items) => self.block(items),
        }
    }

    fn block_last(&mut self, item: &ast::Stmt) -> Option<String> {
        if let ast::Stmt::Expr(expr) = &item
            && expr.semi.is_none()
        {
            match expr.exprs.as_slice() {
                [item] => {
                    let ident = self.push_ident();
                    let expr = self.expr(&item.value);
                    if let Some(expr) = expr {
                        self.result.push_str(&format!("{} = {}\n", ident, expr));
                    }
                    Some(ident)
                }
                [_item, ..] => {
                    for item in &expr.exprs {
                        let ident = self.push_ident();
                        let expr = self.expr(&item.value);
                        if let Some(expr) = expr {
                            self.result.push_str(&format!("{} = {}\n", ident, expr));
                        }
                    }
                    None
                }
                [] => unreachable!(),
            }
        } else {
            let stmt = self.stmt(item);
            self.result.push_str(&format!("{stmt}\n"));
            None
        }
    }

    fn block(&mut self, stmts: &[WithSpan<ast::Stmt>]) -> Option<String> {
        self.push_stack();
        match stmts {
            [item] => self.block_last(&item.value),
            [items @ .., last] => {
                for item in items {
                    let stmt = self.stmt(&item.value);
                    self.result.push_str(&format!("{stmt}\n"));
                }
                self.block_last(&last.value)
            }
            [] => None,
        }
    }
}

pub fn generate(program: &[WithSpan<ast::Stmt>]) -> String {
    let mut context = Context::new();
    context.generate(program);
    context.result
}
