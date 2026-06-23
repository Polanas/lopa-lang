use notify_rust::Notification;

use crate::{
    T, ide,
    parsing::{ast, lexer::Syntax, parser},
};

struct Context {
    output: String,
    ident_level: u32,
}

impl Context {
    fn new() -> Self {
        Self {
            output: String::new(),
            ident_level: 0,
        }
    }

    fn format(&mut self, file: ast::File) {
        for item in file.items() {
            match item {
                ast::Item::FnItem(fn_item) => {
                    self.fn_item(fn_item);
                }
                ast::Item::ModItem(mod_item) => {}
                ast::Item::ImplItem(impl_item) => {}
                ast::Item::StructItem(struct_item) => {}
                ast::Item::EnumItem(enum_item) => {}
                ast::Item::UseItem(use_item) => {}
            }
        }
    }

    fn fn_item(&mut self, fn_item: ast::FnItem) -> Option<()> {
        self.token_space(fn_item.fn_token()?);
        self.text_space(fn_item.name()?.text()?.as_str());

        let _ = self.fn_params(fn_item.params()?);

        Some(())
    }

    fn fn_params(&mut self, params: ast::ParamList) -> Option<()> {
        self.text("(");
        for param in params.params() {
            let _ = self.fn_param(param);
        }
        self.text(")");
        Some(())
    }

    fn fn_param(&mut self, param: ast::FnParam) -> Option<()> {
        // self.token_space(param.);
        Some(())
    }

    fn pattern() {}

    fn with_acc_ident(&mut self, f: impl FnOnce(&mut Self)) {
        self.acc_ident();
        self.with_acc_ident(f);
        self.dec_ident();
    }

    fn dec_ident(&mut self) {
        self.ident_level -= 1;
    }

    fn acc_ident(&mut self) {
        self.ident_level += 1;
    }

    fn token_space(&mut self, token: ast::SyntaxToken) {
        self.token(token);
        self.space();
    }

    fn token(&mut self, token: ast::SyntaxToken) {
        self.text(token.text());
    }

    fn text_space(&mut self, text: &str) {
        self.text(text);
        self.space();
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
    let parse = ide::parse(db, file);

    let mut ctx = Context::new();
    ctx.format(parse.file(db));

    ctx.output
}
