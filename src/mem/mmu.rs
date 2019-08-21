use super::*;

/*
 * MMU struct is responsible for handling address space of CPU.
 * It routes writes/reads to proper places i.e.: RAM in cart or internal VRAM. 
 */
pub struct MMU<T: BankController> {
    /* bootrap contains 256 of boot code. it gets executed first */
    pub bootstrap: Vec<Byte>,
    /* mapper represents the cartdrige and implements its own bank-switching method */
    pub mapper: T,
    /* Different segments of memory map */
    pub vram: Vec<Byte>,
    pub oam: Vec<Byte>,
    pub ram: Vec<Byte>,
    pub hram: Vec<Byte>, 
    pub ioregs: IORegs,
}

impl <T: BankController>MMU<T> {
    pub fn new(mapper: T) -> Self { 
        Self { 
            bootstrap: include_bytes!("data/bootstrap.bin").to_vec(),
            mapper: mapper,
            vram: vec![0; VRAM_SIZE],
            oam: vec![0; OAM_SIZE],
            ram: vec![0; RAM_BANK_SIZE],
            hram: vec![0; HRAM_SIZE],
            ioregs: IORegs::new(),
        }   
    }

    /*
     * WRITEs
     */
    pub fn write(&mut self, addr: Addr, byte: Byte) {
        // BOOTSTRAP ROM | BOOT Sequence
        if addr < BOOSTRAP_SIZE as u16 && self.read(ioregs::BOOT_END) == 0 
            { panic!("Write to 0x{:x} bootstrap ROM.", addr) }

        // BASE ROM | 0x0000-0x3FFF
        else if addr < ROM_SWITCHABLE_ADDR 
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
        else if addr < OAM_ADDR 
            { self.write_base_ram(addr, (addr - RAM_ECHO_ADDR) as usize, byte) }

        // SPRITE ATTRIBUTE TABLE | 0xFE00-0xFEA0 + 0xFEA0-0xFF00(unsued anyway)
        else if addr < IO_REGS_ADDR
            { self.write_oam(addr, (addr - OAM_ADDR) as usize, byte) }
        
        // IO Registers | (0xFF00-0xFF7F + 0xFFFF)
        else if addr < STACK_ADDR || addr == 0xFFFF 
            { self.write_io_reg(addr, (addr - IO_REGS_ADDR) as usize, byte) }
        
        // High RAM - 0xFF80-0xFFFE(hram goes here)
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
        self.hram[offset] = value;
    }

    /*
     * READs
     */
    pub fn read(&mut self, addr: Addr) -> Byte {
        // BOOTSTRAP ROM | BOOT Sequence
        if addr < BOOSTRAP_SIZE as u16 && self.read(ioregs::BOOT_END) == 0
            { self.bootstrap[addr as usize] }
        
        // BASE ROM | 0x0000-0x3FFF
        else if addr < ROM_SWITCHABLE_ADDR 
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
        else if addr < OAM_ADDR 
            { self.read_base_ram(addr, (addr - RAM_ECHO_ADDR) as usize) }

        // SPRITE ATTRIBUTE TABLE | 0xFE00-0xFEA0 + 0xFEA0-0xFF00(unsued anyway)
        else if addr < IO_REGS_ADDR
            { self.read_oam(addr, (addr - OAM_ADDR) as usize) }
        
        // IO Registers | (0xFF00-0xFF7F + 0xFFFF) + 0xFF80 + 0xFFFE(High RAM - unused)
        else if addr < STACK_ADDR || addr == 0xFFFF
            { self.read_io_reg(addr, (addr - IO_REGS_ADDR) as usize) }

        // High RAM - 0xFF80-0xFFFE( hram goes here)
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
        self.hram[offset]
    }
}