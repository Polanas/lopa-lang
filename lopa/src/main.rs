use crop::Rope;
use lopa_core::{
    self,
    parsing::parser::{self, Lang, Prettify},
};
use std::{error::Error, str::FromStr};

fn main() -> Result<(), Box<dyn Error>> {
    let mut rope = Rope::from("h\n");
    let mut s = String::from("h\n");
    dbg!(rope.byte_len(), s.len());
    Ok(())
}
