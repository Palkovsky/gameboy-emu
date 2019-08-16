mod mem;
use mem::*;

fn main() {
    let rom: Vec<Byte> = vec![0; 1<<10];
    let cart_header: Vec<Byte> = rom.iter()
        .take(0x150).map(|x| *x).collect();
    let cart_body: Vec<Byte> = rom.iter()
        .skip(0x150).map(|x| *x).collect();

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

/*
    let mut mapper = mem::mbc::MBC1::new(cart_body);
    mapper.ram[3*RAM_BANK_SIZE] = 0x69;  
    mapper.ram[2*RAM_BANK_SIZE+1] = 0x70;

    mapper.rom[21*ROM_BANK_SIZE] = 0x11;
    mapper.rom[66*ROM_BANK_SIZE] = 0x22;  
    mapper.rom[88*ROM_BANK_SIZE+3] = 0x33;  
    
    let mut memory = mem::Memory::new(mapper);
    memory.write(0x0000, 0x0A); // Enable RAM
    memory.write(0x6000, 1); // Enable 4 RAM banks mode

    memory.write(0x4000, 0x3);  // Select 3rd RAM bank
    println!("{:x}", memory.read(RAM_SWITCHABLE_ADDR));

    memory.write(0x4000, 0x2);  // Select 2nd RAM bank
    println!("{:x}", memory.read(RAM_SWITCHABLE_ADDR+1));

    memory.write(0x2000, 21); // Select 21st ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR));

    memory.write(0x2000, 66); // Select 66th ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR));

    memory.write(0x6000, 0); // Enable 1 RAM bank mode
    memory.write(0x2000, 66); // Select 66th ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR));

    memory.write(0x2000, 88); // Select 88th ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR + 3));
*/
}
