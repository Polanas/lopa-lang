#[allow(unused)]
mod shared_mut;

use std::{error::Error, io::Read};

use lopa_lang::{code_gen, parser, tokenizer, types};
fn main() -> Result<(), Box<dyn Error>> {
    let source = "x = 3;
        if x == 1 {
            print 1;
        } else if x == 2 {
            print 2;
        } else {
            print 3;
        }";

    let tokens = tokenizer::tokenize(source);
    let ast = parser::parse_program(&tokens);
    match ast {
        Ok(mut ast) => {
            let mut type_context = types::Context::new();
            // if type_context.type_check(&mut ast, source).is_none() {
            //     for error in type_context.diagnostics {
            //         dbg!(
            //             error.message,
            //             Some(
            //                 &source[(((error.span.start.0 as i32) - 2).min(0) as usize)
            //                     ..(error.span.end.0) + 2]
            //             )
            //         );
            //     }
            // }
            let code = code_gen::generate(&ast);
            println!("{code}");
            println!("------------------------------");
            mlua::Lua::new().load(&code).exec().unwrap();
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
