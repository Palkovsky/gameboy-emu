mod mem;
use mem::*;

fn main() {
    let rom: Vec<Byte> = vec![0; 1<<10];
    let cart_header: Vec<Byte> = rom.iter()
        .take(0x150).map(|x| *x).collect();
    let cart_body: Vec<Byte> = rom.iter()
        .skip(0x150).map(|x| *x).collect();
}
