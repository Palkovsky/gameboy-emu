pub mod mem;
pub use mem::*;
pub mod utils;
pub use utils::*;

use std::{env, fs, io};
use io::prelude::*;

fn main() -> io::Result<()> {
    if env::args().len() != 2 {
        panic!("Usage: {} [rom]", env::args().nth(0).unwrap());
    }

    let path = env::args().nth(1).unwrap();
    let mut file = fs::File::open(path).unwrap();

    let mut rom = Vec::new();
    file.read_to_end(&mut rom)?;

    let header: Vec<u8> = rom.iter()
        .take(0x150).skip(0x100)
        .map(|x| *x).collect();
    let header = CartHeader::new(header);
    
    println!("{}", header);

    Ok(())
}
