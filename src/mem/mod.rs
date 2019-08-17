pub mod mbc;
pub mod ioregs;

use mbc::*;
pub use ioregs::*;

pub type Addr = u16;
pub type Byte = u8;
pub type MutMem<'a> = &'a mut [Byte];

/*
 * Base addresses of different memory map segments.
 */ 
pub const ROM_BASE_ADDR: Addr = 0x0000;
pub const ROM_SWITCHABLE_ADDR: Addr = 0x4000;
pub const VRAM_ADDR: Addr = 0x8000;
pub const RAM_SWITCHABLE_ADDR: Addr = 0xA000;
pub const RAM_BASE_ADDR: Addr = 0xC000;
pub const RAM_ECHO_ADDR: Addr = 0xE000;
pub const SPRITE_ATTRIBUTE_ADDR: Addr = 0xFE00;
pub const STACK_ADDR: Addr = 0xFF80;
pub const IO_REGS_ADDR: Addr = 0xFF00;

pub const RAM_BANK_SIZE: usize = 0x2000;
pub const ROM_BANK_SIZE: usize = 0x4000;
pub const VRAM_SIZE: usize = 0x2000;
pub const INTERNAL_SIZE: usize = 0x200;
pub const OAM_SIZE: usize = 0xA0;
pub const IO_REG_SIZE: usize = 0x80;
pub const STACK_SIZE: usize = 0x80;

/*
 * Memory struct is responsible for handling address space of CPU.
 * It routes writes/reads to proper places i.e.: RAM in cart or internal VRAM. 
 */
pub struct Memory<T: BankController> {
    pub mapper: T,
    pub vram: Vec<Byte>,
    pub ram: Vec<Byte>,
    pub oam: Vec<Byte>,
    pub stack: Vec<Byte>, 
    pub ioregs: IORegs,
}

impl <T: BankController>Memory<T> {
    pub fn new(mapper: T) -> Self { 
        Self { mapper: mapper,
               vram: vec![0; VRAM_SIZE],
               ram: vec![0; RAM_BANK_SIZE],
               oam: vec![0; OAM_SIZE],
               stack: vec![0; STACK_SIZE],
               ioregs: IORegs::new(),
        }   
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

        // SPRITE ATTRIBUTE TABLE | 0xFE00-0xFEA0 + 0xFEA0-0xFF00(unsued anyway)
        else if addr < IO_REGS_ADDR
            { self.write_oam(addr, (addr - SPRITE_ATTRIBUTE_ADDR) as usize, byte) }
        
        // IO Registers | (0xFF00-0xFF7F + 0xFFFF)
        else if addr < STACK_ADDR || addr == 0xFFFF 
            { self.write_io_reg(addr, (addr - IO_REGS_ADDR) as usize, byte) }
        
        // High RAM - 0xFF80-0xFFFE(stack goes here)
        else 
            { self.write_to_stack(addr, (addr - STACK_ADDR) as usize, byte) }
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
        self.ram[offset] = value;
    }

    fn write_oam(&mut self, _: Addr, offset: usize, value: Byte) {
        self.oam[offset] = value;
    }

    fn write_io_reg(&mut self, _: Addr, offset: usize, value: Byte) {
        self.ioregs.slice()[offset] = value;
    }

    fn write_to_stack(&mut self, _: Addr, offset: usize, value: Byte) {
        self.stack[offset] = value;
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

        // SPRITE ATTRIBUTE TABLE | 0xFE00-0xFEA0 + 0xFEA0-0xFF00(unsued anyway)
        else if addr < IO_REGS_ADDR
            { self.read_oam(addr, (addr - SPRITE_ATTRIBUTE_ADDR) as usize) }
        
        // IO Registers | (0xFF00-0xFF7F + 0xFFFF) + 0xFF80 + 0xFFFE(High RAM - unused)
        else if addr < STACK_ADDR || addr == 0xFFFF
            { self.read_io_reg(addr, (addr - IO_REGS_ADDR) as usize) }

        // High RAM - 0xFF80-0xFFFE( stack goes here)
        else 
            { self.read_stack(addr, (addr - STACK_ADDR) as usize) }
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
        self.ram[offset]
    }

    fn read_oam(&mut self, _: Addr, offset: usize) -> Byte {
        self.oam[offset]
    }

    fn read_io_reg(&mut self, _: Addr, offset: usize) -> Byte {
        self.ioregs.slice()[offset]
    }

    fn read_stack(&mut self, _: Addr, offset: usize) -> Byte {
        self.stack[offset]
    }
}