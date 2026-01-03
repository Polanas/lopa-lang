#[allow(unused)]
mod shared_mut;

use std::{error::Error, io::Read};

use lopa_lang::{code_gen, parser, position, tokenizer, types};
fn main() -> Result<(), Box<dyn Error>> {
    let source = "
            fn test(a: int, b: int) -> int, int {
                {1, {2}}
            }
";
    let tokens = tokenizer::tokenize(source);
    let ast = parser::parse_program(&tokens);
    match ast {
        Ok(mut ast) => {
            dbg!(ast);
            // let mut type_context = types::Context::new();
            // let code = code_gen::generate(&ast);
            // println!("------------------------------");
            // println!("{code}");
            // type_context.type_check(&mut ast, source);
            // if !type_context.diagnostics.is_empty() {
            //     let offsets = position::LineOffsets::new(source);
            //     for error in type_context.diagnostics {
            //         println!(
            //             "[ERROR on line {}]: {}",
            //             offsets.line(error.span.start),
            //             error.message
            //         );
            //         // dbg!(
            //         //     error.message,
            //         //     Some(
            //         //         &source[(((error.span.start.0 as i32) - 2).min(0) as usize)
            //         //             ..(error.span.end.0) + 2]
            //         //     )
            //         // );
            //     }
            // } else {
            //     let code = code_gen::generate(&ast);
            //     println!("------------------------------");
            //     println!("{code}");
            //     mlua::Lua::new().load(&code).exec().unwrap();
            // }
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
