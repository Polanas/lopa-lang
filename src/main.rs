#[allow(unused)]
mod shared_mut;

use std::error::Error;

use lopa_lang::{code_gen, parser, position, tokenizer, types};
fn main() -> Result<(), Box<dyn Error>> {
    let source = r#"
extern(lua) {
    fn print(v1: any, v2: any, v3: any)
}
fn test() -> int, int {
    1,2
}
fn main() {
    let x,y,z = test(),3;
    print(x,y,z);
}
"#;
    let tokens = tokenizer::tokenize(source);
    let ast = parser::parse_program(&tokens);
    match ast {
        Ok(mut ast) => {
            let mut type_context = types::Context::new(source);
            type_context.type_check(&mut ast);
            if !type_context.diagnostics.is_empty() {
                let offsets = position::LineOffsets::new(source);
                for error in type_context.diagnostics {
                    println!(
                        "[ERROR on line {}]: {}",
                        offsets.line(error.span.start),
                        error.message
                    );
                }
            } else {
                let code = code_gen::generate(&ast);
                println!("------------------------------");
                println!("{code}");
                let lua = mlua::Lua::new();
                lua.load(&code).exec().unwrap();
                lua.load("main()").exec().unwrap();
            }
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
