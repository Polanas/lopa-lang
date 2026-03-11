#[allow(unused)]
use std::error::Error;

use lopa_lang::{parser, position, tokenizer, type_check};
fn main() -> Result<(), Box<dyn Error>> {
    //TODO: unknown types as fn params are not checked
    let source = r#"
        fn main() {
            let x: int? = nil;
        }
    "#;
    let tokens = tokenizer::tokenize(source);
    let ast = parser::parse_program(&tokens);
    match ast {
        Ok(ast) => {
            let mut type_context = type_check::TypeCheck::new();
            type_context.set_source(source);
            type_context.check(&ast);
            if !type_context.diagnostics().is_empty() {
                let offsets = position::LineOffsets::new(source);
                for error in type_context.diagnostics() {
                    println!(
                        "[ERROR on line {}]: {}",
                        offsets.line(error.span.start),
                        error.message
                    );
                }
            } else {
                // let code = code_gen::generate(&ast);
                // println!("------------------------------");
                // println!("{code}");
                // let lua = mlua::Lua::new();
                // lua.load(&code).exec().unwrap();
                // lua.load("main()").exec().unwrap();
            }
            type_context.debug_dump();
        }
        Err(errs) => {
            for error in errs {
                println!("ERROR: {}", error.message);
                println!(
                    "{}",
                    &source[(((error.span.start.0 as i32) - 2).min(0) as usize)
                        ..((error.span.end.0) + 2).min(source.len())]
                );
            }
        }
    }
    Ok(())
}
