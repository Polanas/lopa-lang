mod compiler;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let app_m = clap::Command::new("lopa")
        .version("0.0.1")
        .about("Lopa language compiler")
        .subcommand(clap::Command::new("run").about("Runs a lopa project"))
        .get_matches();

    match app_m.subcommand() {
        Some(("run", _sub_m)) => {

        }
        _ => {}
    }
    Ok(())
}
