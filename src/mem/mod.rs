pub mod mbc;
use mbc::*;

pub type Addr = u16;
pub type Byte = u8;
pub type MutMem<'a> = &'a mut [Byte];

pub const ROM_BASE_ADDR: u16 = 0x0000;
pub const ROM_SWITCHABLE_ADDR: u16 = 0x4000;
pub const VRAM_ADDR: u16 = 0x8000;
pub const RAM_SWITCHABLE_ADDR: u16 = 0xA000;
pub const RAM_BASE_ADDR: u16 = 0xC000;
pub const RAM_ECHO_ADDR: u16 = 0xE000;
pub const SPRITE_ATTRIBUTE_ADDR: u16 = 0xFE00;

pub const RAM_BANK_SIZE: usize = 0x2000;
pub const ROM_BANK_SIZE: usize = 0x4000;
pub const VRAM_SIZE: usize = 0x2000;
pub const INTERNAL_SIZE: usize = 0x200;

pub struct Memory<T: BankController> {
    mapper: T,
    vram: Vec<Byte>,
    internal: Vec<Byte>,
}

impl <T: BankController>Memory<T> {
    pub fn new(mapper: T) -> Self { 
        Self { mapper: mapper, vram: vec![0; VRAM_SIZE], internal: vec![0; INTERNAL_SIZE]} 
    }

    /*
     * WRITEs
     */
    pub fn write(&mut self, addr: Addr, byte: Byte) {
        // BASE ROM | 0x0000-0x3FFF
        if addr < ROM_SWITCHABLE_ADDR 
            { self.write_base_rom(addr, addr as usize, byte) } 
        // SWITCHABLE ROM | 0x4000-0x7FFF
        else if addr < VRAM_ADDR 
            { self.write_switchable_rom(addr, (addr - ROM_SWITCHABLE_ADDR) as usize, byte) }
         // VRAM | 0x8000-0x9FFF
        else if addr < RAM_SWITCHABLE_ADDR 
            { self.write_vram(addr, (addr - VRAM_ADDR) as usize, byte) } 
         // SWITCHABLE RAM | 0xA000-0xBFFF
        else if addr < RAM_BASE_ADDR 
            { self.write_switchable_ram(addr, (addr - RAM_SWITCHABLE_ADDR) as usize, byte) }
        // BASE RAM | 0xC000 - 0xDFFF
        else if addr < RAM_ECHO_ADDR 
            { self.write_base_ram(addr, (addr - RAM_BASE_ADDR) as usize, byte) }
        // ECHO OF BASE RAM | 0xE000 - 0xFE00
        else if addr < SPRITE_ATTRIBUTE_ADDR 
            { self.write_base_ram(addr, (addr - RAM_ECHO_ADDR) as usize, byte) }
        // Rest 0xFE00-0xFFFF
        else 
            { self.write_internal(addr, (addr - SPRITE_ATTRIBUTE_ADDR) as usize, byte) }
    }

    fn write_base_rom(&mut self, addr: Addr, _: usize, value: Byte) {
        match self.mapper.get_addr_type(addr) {
            AddrType::Status =>  self.mapper.on_status(addr, value),
            AddrType::Write => panic!("Attempt to write to ROM at 0x{:X}", addr),
        }
    }

    fn write_switchable_rom(&mut self, addr: Addr, _: usize, value: Byte) {
        match self.mapper.get_addr_type(addr) {
            AddrType::Status => self.mapper.on_status(addr, value),
            AddrType::Write => panic!("Attempt to write to ROM at 0x{:X}", addr),
        }
    }

    fn write_vram(&mut self, _: Addr, offset: usize, value: Byte) { 
        self.vram[offset] = value;
    }

    fn write_switchable_ram(&mut self, addr: Addr, offset: usize, value: Byte) {
        match self.mapper.get_addr_type(addr) {
            AddrType::Status => panic!("Unable to send status at RAM address 0x{:X}", addr),
            AddrType::Write => self.mapper.get_switchable_ram().unwrap()[offset] = value,
        }
    }

    fn write_base_ram(&mut self, addr: Addr, offset: usize, value: Byte) {
        match self.mapper.get_addr_type(addr) {
            AddrType::Status => panic!("Unable to send status at RAM address 0x{:X}", addr),
            AddrType::Write => self.mapper.get_base_ram().unwrap()[offset] = value,
        }    
    }

    fn write_internal(&mut self, _: Addr, offset: usize, value: Byte) {
        self.internal[offset] = value;
    }

    /*
     * READs
     */
    pub fn read(&mut self, addr: Addr) -> Byte {
        // BASE ROM | 0x0000-0x3FFF
        if addr < ROM_SWITCHABLE_ADDR 
            { self.read_base_rom(addr, addr as usize) } 
        // SWITCHABLE ROM | 0x4000-0x7FFF
        else if addr < VRAM_ADDR 
            { self.read_switchable_rom(addr, (addr - ROM_SWITCHABLE_ADDR) as usize) }
         // VRAM | 0x8000-0x9FFF
        else if addr < RAM_SWITCHABLE_ADDR 
            { self.read_vram(addr, (addr - VRAM_ADDR) as usize) } 
         // SWITCHABLE RAM | 0xA000-0xBFFF
        else if addr < RAM_BASE_ADDR 
            { self.read_switchable_ram(addr, (addr - RAM_SWITCHABLE_ADDR) as usize) }
        // BASE RAM | 0xC000 - 0xDFFF
        else if addr < RAM_ECHO_ADDR 
            { self.read_base_ram(addr, (addr - RAM_BASE_ADDR) as usize) }
        // ECHO OF BASE RAM | 0xE000 - 0xFE00
        else if addr < SPRITE_ATTRIBUTE_ADDR 
            { self.read_base_ram(addr, (addr - RAM_ECHO_ADDR) as usize) }
        // Rest 0xFE00-0xFFFF
        else 
            { self.read_internal(addr, (addr - SPRITE_ATTRIBUTE_ADDR) as usize) }
    }

    fn read_base_rom(&mut self, _: Addr, offset: usize) -> Byte {
        self.mapper.get_base_rom().unwrap()[offset]
    }

    fn read_switchable_rom(&mut self, _: Addr, offset: usize) -> Byte {
        self.mapper.get_switchable_rom().unwrap()[offset]
    }

    fn read_vram(&mut self, _: Addr, offset: usize) -> Byte { 
        self.vram[offset] 
    }

    fn read_switchable_ram(&mut self, _: Addr, offset: usize) -> Byte {
        self.mapper.get_switchable_ram().unwrap()[offset]
    }

    fn read_base_ram(&mut self, _: Addr, offset: usize) -> Byte {
        self.mapper.get_base_ram().unwrap()[offset]
    }

    fn read_internal(&mut self, _: Addr, offset: usize) -> Byte {
        self.internal[offset]
    }
}