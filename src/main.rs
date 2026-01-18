#[allow(unused)]
mod shared_mut;

use std::error::Error;

use lopa_lang::{code_gen, parser, position, tokenizer, types};
fn main() -> Result<(), Box<dyn Error>> {
    let source = r#"
inline(lua) {
    fn do_stuff(value: int?) -> int = """
        value = value or 0
        return value + 20
    """
}

extern(lua) {
    fn print(value: any)
}

fn print_wrapper(print_fn: fn(idk: any)) {
    print_fn(idk: 20);
}

fn main() {
    print_wrapper(print);
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
                    // dbg!(
                    //     error.message,
                    //     Some(
                    //         &source[(((error.span.start.0 as i32) - 2).min(0) as usize)
                    //             ..(error.span.end.0) + 2]
                    //     )
                    // );
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
                dbg!(
                    error.message,
                    Some(
                        &source[(((error.span.start.0 as i32) - 2).min(0) as usize)
                            ..(error.span.end.0) + 2]
                    )
                );
            }
        }
    }

    Ok(())
}
