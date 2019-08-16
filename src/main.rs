mod mem;
use mem::*;

fn main() {
    let rom: Vec<Byte> = vec![0; 1<<10];
    let cart_header: Vec<Byte> = rom.iter()
        .take(0x150).map(|x| *x).collect();
    let cart_body: Vec<Byte> = rom.iter()
        .skip(0x150).map(|x| *x).collect();

 /*
    let mut mapper = mem::mbc::MBC2::new(cart_body);
    mapper.ram[128] = 0xFF;  
    mapper.ram[1] = 0x2E;

    mapper.rom[0x5*ROM_BANK_SIZE] = 0x11;
    mapper.rom[0x7*ROM_BANK_SIZE] = 0x22;  
    mapper.rom[0xF*ROM_BANK_SIZE+3] = 0x33;  
    
    let mut memory = mem::Memory::new(mapper);
    memory.write(0x0000, 0x0A); // Enable RAM

    println!("0x{:x}", memory.read(RAM_SWITCHABLE_ADDR + 128));
    println!("0x{:x}", memory.read(RAM_SWITCHABLE_ADDR + 1));

    memory.write(0x2100, 0x5);  // Select 5th ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR));

    memory.write(0x2300, 0xF); // Select 15th ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR + 3));
*/
}
