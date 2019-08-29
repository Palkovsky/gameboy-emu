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
    /* Statistics */
    pub writes: u64,
    pub reads: u64,
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
            writes: 0, reads: 0,
        }   
    }

    /* Allows setting bit in memory byte. n of 0 means least signifcant bit */
    pub fn set_bit(&mut self, addr: Addr, n: u8, flg: bool) {
        let byte = self.read(addr);
        
        let mask = 1u8 << n;
        let num = if flg { 1 } else { 0 };
        let updated = (byte & !mask) | ((num << n) & mask);

        self.write(addr, updated);
    }

    /* Allows reading nth bit */
    pub fn read_bit(&mut self, addr: Addr, n: u8) -> bool {
        let byte = self.read(addr);
        byte & (1 << n) != 0
    }

    /* WRITES */
    pub fn write(&mut self, addr: Addr, byte: Byte) {
        self.writes += 1;

        if addr < BOOSTRAP_SIZE as u16 && self.read(ioregs::BOOT) == 0x00 {
            panic!("Attempt to write to bootstrap ROM at 0x{:X}", addr)
        }
        
        // The thing below is quite retarded, but I was hoping for some magic optimalizations.
        let chunked = ((addr >> 12) & 0xF, (addr >> 8) & 0xF, (addr >> 4) & 0xF, addr & 0xF);
        match chunked {
            (0, _, _, _) | (1, _, _, _) | (2, _, _, _) | (3, _, _, _) => 
                self.write_base_rom(addr, addr as usize, byte),
            (4, _, _, _) | (5, _, _, _) | (6, _, _, _) | (7, _, _, _) => 
                self.write_switchable_rom(addr, (addr - ROM_SWITCHABLE_ADDR) as usize, byte),
            (8, _, _, _) | (9, _, _, _) => 
                self.write_vram(addr, (addr - VRAM_ADDR) as usize, byte),
            (10, _, _, _) | (11, _, _, _) => 
                self.write_switchable_ram(addr, (addr - RAM_SWITCHABLE_ADDR) as usize, byte),
            (12, _, _, _) | (13, _, _, _) => 
                self.write_base_ram(addr, (addr - RAM_BASE_ADDR) as usize, byte),
            (14, _, _, _) |            
            (15, 0, _, _) | (15, 1, _, _) | (15, 2, _, _) | (15, 3, _, _)  | (15, 4, _, _) | (15, 5, _, _) | (15, 6, _, _) |
            (15, 7, _, _) | (15, 8, _, _) | (15, 9, _, _) | (15, 10, _, _) | (15, 11, _, _) | (15, 12, _, _) | (15, 13, _, _)  =>
                self.write_base_ram(addr, (addr - RAM_ECHO_ADDR) as usize, byte),
            (15, 14, _, _) => 
                self.write_oam(addr, (addr - OAM_ADDR) as usize, byte),
            (15, 15, 0, _) | (15, 15, 1, _) | (15, 15, 2, _) | (15, 15, 3, _) | (15, 15, 4, _) | (15, 15, 5, _) | (15, 15, 6, _) |
            (15, 15, 7, _) | (15, 15, 15, 15) =>
                self.write_io_reg(addr, (addr - IO_REGS_ADDR) as usize, byte),
            (15, 15, _, _) => self.write_hram(addr, (addr - HRAM_ADDR) as usize, byte),
            _ => panic!("Unmapped address 0x{:x}", addr),
        };
    }

    fn write_base_rom(&mut self, addr: Addr, _: usize, value: Byte) {
        match self.mapper.get_addr_type(addr) {
            AddrType::Status =>  self.mapper.on_status(addr, value),
            AddrType::Write => println!("Attempt to write to ROM at 0x{:X}", addr),
        }
    }

    fn write_switchable_rom(&mut self, addr: Addr, _: usize, value: Byte) {
        match self.mapper.get_addr_type(addr) {
            AddrType::Status => self.mapper.on_status(addr, value),
            AddrType::Write => println!("Attempt to write to ROM at 0x{:X}", addr),
        }
    }

    fn write_vram(&mut self, _: Addr, offset: usize, value: Byte) { 
        self.vram[offset] = value;
    }

    fn write_switchable_ram(&mut self, addr: Addr, offset: usize, value: Byte) {
        println!("{:x}", addr);
        match self.mapper.get_addr_type(addr) {
            AddrType::Status => panic!("Unable to send status at RAM address 0x{:X}", addr),
            AddrType::Write => match self.mapper.get_switchable_ram() {
                None => println!("Attempted to write to 0x{:x}, storage not present.", addr),
                Some(arr) => arr[offset] = value,
            }
        }
    }

    fn write_base_ram(&mut self, _: Addr, offset: usize, value: Byte) {
        self.ram[offset] = value;
    }

    fn write_oam(&mut self, _: Addr, offset: usize, value: Byte) {
        self.oam[offset] = value;
    }

    fn write_io_reg(&mut self, _: Addr, offset: usize, value: Byte) {
        self.ioregs.slice()[offset] = value;
    }

    fn write_hram(&mut self, _: Addr, offset: usize, value: Byte) {
        self.hram[offset] = value;
    }

    /* READS */
    pub fn read(&mut self, addr: Addr) -> Byte {
        self.reads += 1;

        if addr < BOOSTRAP_SIZE as u16 && self.read(ioregs::BOOT) == 0x00 {
            return self.bootstrap[addr as usize]
        }

        // The thing below is quite retarded, but I was hoping for some magic optimalizations.
        let chunked = ((addr >> 12) & 0xF, (addr >> 8) & 0xF, (addr >> 4) & 0xF, addr & 0xF);
        match chunked {
            (0, _, _, _) | (1, _, _, _) | (2, _, _, _) | (3, _, _, _) => 
                self.read_base_rom(addr, addr as usize),
            (4, _, _, _) | (5, _, _, _) | (6, _, _, _) | (7, _, _, _) => 
                self.read_switchable_rom(addr, (addr - ROM_SWITCHABLE_ADDR) as usize),
            (8, _, _, _) | (9, _, _, _) => 
                self.read_vram(addr, (addr - VRAM_ADDR) as usize),
            (10, _, _, _) | (11, _, _, _) => 
                self.read_switchable_ram(addr, (addr - RAM_SWITCHABLE_ADDR) as usize),
            (12, _, _, _) | (13, _, _, _) => 
                self.read_base_ram(addr, (addr - RAM_BASE_ADDR) as usize),
            (14, _, _, _) |            
            (15, 0, _, _) | (15, 1, _, _) | (15, 2, _, _) | (15, 3, _, _)  | (15, 4, _, _) | (15, 5, _, _) | (15, 6, _, _) |
            (15, 7, _, _) | (15, 8, _, _) | (15, 9, _, _) | (15, 10, _, _) | (15, 11, _, _) | (15, 12, _, _) | (15, 13, _, _)  =>
                self.read_base_ram(addr, (addr - RAM_ECHO_ADDR) as usize),
            (15, 14, _, _) => 
                self.read_oam(addr, (addr - OAM_ADDR) as usize),
            (15, 15, 0, _) | (15, 15, 1, _) | (15, 15, 2, _) | (15, 15, 3, _) | (15, 15, 4, _) | (15, 15, 5, _) | (15, 15, 6, _) |
            (15, 15, 7, _) | (15, 15, 15, 15) =>
                self.read_io_reg(addr, (addr - IO_REGS_ADDR) as usize),
            (15, 15, _, _) => self.read_hram(addr, (addr - HRAM_ADDR) as usize),
            _ => panic!("Unmapped address 0x{:x}", addr),
        }
    }

    fn read_base_rom(&mut self, addr: Addr, offset: usize) -> Byte {
        match self.mapper.get_base_rom() {
            Some(arr) => return arr[offset],
            None => { println!("Attempted to read unexistent memory at 0x{:x}", addr); 0xFF },
        }
    }

    fn read_switchable_rom(&mut self, addr: Addr, offset: usize) -> Byte {
        match self.mapper.get_switchable_rom() {
            Some(arr) => return arr[offset],
            None => { println!("Attempted to read unexistent memory at 0x{:x}", addr); 0xFF },
        }
    }

    fn read_vram(&mut self, _: Addr, offset: usize) -> Byte { 
        self.vram[offset] 
    }

    fn read_switchable_ram(&mut self, addr: Addr, offset: usize) -> Byte {
        match self.mapper.get_switchable_ram() {
            Some(arr) => return arr[offset],
            None => { println!("Attempted to read unexistent memory at 0x{:x}", addr); 0xFF },
        }
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

    fn read_hram(&mut self, _: Addr, offset: usize) -> Byte {
        self.hram[offset]
    }

    pub fn disable_bootrom(&mut self) { self.write(ioregs::BOOT, 1); }
}