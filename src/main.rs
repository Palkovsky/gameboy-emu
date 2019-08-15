const ROM_BASE_ADDR: u16 = 0x0000;
const ROM_SWITCHABLE_ADDR: u16 = 0x4000;
const VRAM_ADDR: u16 = 0x8000;
const RAM_SWITCHABLE_ADDR: u16 = 0xA000;
const RAM_BASE_ADDR: u16 = 0xC000;
const RAM_ECHO_ADDR: u16 = 0xE000;
const SPRITE_ATTRIBUTE_ADDR: u16 = 0xFE00;

const RAM_BANK_SIZE: usize = 0x2000;
const ROM_BANK_SIZE: usize = 0x4000;
const VRAM_SIZE: usize = 0x2000;
const INTERNAL_SIZE: usize = 0x200;

type Addr = u16;
type Byte = u8;
type MutMem<'a> = &'a mut [Byte];

struct Memory<T: BankController> {
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
/*
 * AddrType is used by BankController to determine address type: wheater it is
 * will change MBC registers or perform bank switching or is just regular writable.
 */
#[derive(Copy, Clone)]
enum AddrType {
    Write,
    Status,
}
/*
 * BankController trait represents memory mapper interface.
 */
trait BankController {
    /*
     * Checks whether the addr is special memory region for
     * MBC configuration(setting registers, enabling RAM etc.). 
     */
    fn get_addr_type(&self, addr: Addr) -> AddrType;
    /* Called when get_addr_type() returned Status addr type. */
    fn on_status(&mut self, addr: Addr, value: Byte);
    /* Gets base non-switchable ROM. 0x0000-0x4000 range */
    fn get_base_rom(&mut self) -> Option<MutMem>;
    /* Gets switchable ROM. 0x4000-0x8000 range */
    fn get_switchable_rom(&mut self) -> Option<MutMem>;
    /* Gets switchable RAM. 0xA000-0xC000 range */
    fn get_switchable_ram(&mut self) -> Option<MutMem>;
    /*  Gets base non-switchable RAM. 0xC000-0xE000 */
    fn get_base_ram(&mut self) -> Option<MutMem>;
}

/*
 * Simplest MBC - no switching needed. This implementation assumes that switchable RAM
 * bank(0xA000-0xBFFF) is available.
 */
const ROM_ONLY_SIZE: usize = 1 << 15;
struct RomOnly {
    rom_banks: Vec<Byte>,
}
impl RomOnly {
    fn new(rom: Vec<Byte>) -> Self { 
        let mut mbc = Self { rom_banks: vec![0; ROM_ONLY_SIZE] };
        if rom.len() > mbc.rom_banks.len() { panic!("ROM too big for RomOnly"); }
        for (i, byte) in rom.into_iter().enumerate() { mbc.rom_banks[i] = byte; }
        mbc
    }
}
impl BankController for RomOnly {
    fn get_addr_type(&self, addr: Addr) -> AddrType { AddrType::Write }    
    fn on_status(&mut self, _: Addr, _: Byte) {}
    fn get_base_rom(&mut self) -> Option<MutMem> { Some(&mut self.rom_banks[..ROM_BANK_SIZE]) }
    fn get_switchable_rom(&mut self) -> Option<MutMem> { None }
    fn get_base_ram(&mut self) -> Option<MutMem> { None }
    fn get_switchable_ram(&mut self) -> Option<MutMem> { Some(&mut self.rom_banks[ROM_BANK_SIZE..ROM_BANK_SIZE*2])}
}

const MBC1_MAX_RAM_BANKS: usize = 4;
const MBC1_MAX_ROM_BANKS: usize = 128;
const RAM_DISABLED: u8 = 0;
const RAM_ENABLED: u8 = 1;
const RAM_MODE: u8 = 1;
const ROM_MODE: u8 = 0;

struct MBC1 {
    ram_banks: Vec<Byte>,
    rom_banks: Vec<Byte>,
    ram_enabled: u8,
    banking_mode: u8,
    idx: u8,
}
impl MBC1 {
    fn new(rom: Vec<Byte>) -> Self { 
        let mut mbc = Self {
            ram_banks: vec![0; RAM_BANK_SIZE*MBC1_MAX_RAM_BANKS],
            rom_banks: vec![0; ROM_BANK_SIZE*MBC1_MAX_ROM_BANKS],
            ram_enabled: RAM_ENABLED, banking_mode: ROM_MODE,
            idx: 0,
        }; 
        if rom.len() > mbc.rom_banks.len() { panic!("ROM too big for MBC1"); }
        for (i, byte) in rom.into_iter().enumerate() { mbc.rom_banks[i] = byte; }
        mbc
    }
}
impl BankController for MBC1 {
    fn get_addr_type(&self, addr: Addr) -> AddrType {
        let intervals = [
            (0x0000, 0x1FFF),  // RAM enable
            (0x6000, 0x7FFF),  // ROM/RAM banking mode
            (0x2000, 0x3FFF), // ROM bank swap
            (0x4000, 0x5FFF), // RAM/ROM bank swap
        ];
        for (start, end) in intervals.iter() {
            if addr >= *start && addr <= *end { return AddrType::Status }
        }
        AddrType::Write
    }    
    fn on_status(&mut self, addr: Addr, value: Byte) {
        // 0x0000 - 0x2000 -> RAM ON/OFF
        // To enable: XXXX1010
        if addr < 0x2000 { 
            self.ram_enabled = if value & 0xF == 0xA { RAM_ENABLED } else { RAM_DISABLED };
        }
        // 0x2000-0x4000 - ROM bank switch
        // Bank idx: XXXBBBBB
        if addr >= 0x2000 && addr < 0x4000 {
            self.idx = (value & 0b00011111) | (self.idx & 0b11100000);
            if self.idx & 0x1F == 0 { self.idx += 1; } // If 5 lower bits are zeros => change to 1
        }
        // 0x4000-0x6000 - ROM/RAM bank switch
        // XXXXXXBB
        if addr >= 0x4000 && addr < 0x6000 {
            self.idx = ((value & 0x3) << 5) | (self.idx & 0b10011111);
            if self.idx & 0x1F == 0 { self.idx += 1; } // If 5 lower bits are zeros => change to 1
        }
        // 0x6000 - 0x8000 -> Banking Mode(RAM/ROM)
        // For ROM(8KB RAM, 2MB ROM): XXXXXXX1, for RAM(32KB RAM, 512KB ROM): XXXXXXX0
        if addr >= 0x6000 && addr < 0x8000 {
            self.banking_mode = value & 1;
        }
    }

    fn get_base_rom(&mut self) -> Option<MutMem> { Some(&mut self.rom_banks[..ROM_BANK_SIZE]) }
    fn get_switchable_rom(&mut self) -> Option<MutMem> {
        let rom_idx = self.idx 
            & if self.banking_mode == ROM_MODE { 0x7F } else { 0x1F };
        
        let start = (rom_idx as usize) * ROM_BANK_SIZE;
        let end = start + ROM_BANK_SIZE;
        Some(&mut self.rom_banks[start..end])
    }
    fn get_base_ram(&mut self) -> Option<MutMem> { Some(&mut self.ram_banks[..RAM_BANK_SIZE]) }
    fn get_switchable_ram(&mut self) -> Option<MutMem> {
        let ram_idx = (self.idx 
            & if self.banking_mode == RAM_MODE { 0b01100000 } else { 0 }) >> 5;
        let start = (ram_idx as usize) * RAM_BANK_SIZE;
        let end = start + RAM_BANK_SIZE;
        Some(&mut self.ram_banks[start..end])
    }
}

fn main() {
    // Mock of rom ROM
    let rom: Vec<Byte> = vec![0; 1<<10];
    let cart_header: Vec<Byte> = rom.iter()
        .take(0x150).map(|x| *x).collect();
    let cart_body: Vec<Byte> = rom.iter()
        .skip(0x150).map(|x| *x).collect();

    let mut mapper = MBC1::new(cart_body);
    mapper.ram_banks[3*RAM_BANK_SIZE] = 0x69;  
    mapper.ram_banks[2*RAM_BANK_SIZE+1] = 0x70;

    mapper.rom_banks[21*ROM_BANK_SIZE] = 0x11;
    mapper.rom_banks[66*ROM_BANK_SIZE] = 0x22;  
    mapper.rom_banks[88*ROM_BANK_SIZE+3] = 0x33;  
    
    let mut memory = Memory::new(mapper);
    memory.write(0x0000, 0x0A); // Enable RAM
    memory.write(0x6000, 1); // Enable 4 RAM banks mode

    memory.write(0x4000, 0x3);  // Select 3rd RAM bank
    println!("{:x}", memory.read(RAM_SWITCHABLE_ADDR));

    memory.write(0x4000, 0x2);  // Select 2nd RAM bank
    println!("{:x}", memory.read(RAM_SWITCHABLE_ADDR+1));

    memory.write(0x2000, 21); // Select 21st ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR));

    memory.write(0x2000, 66); // Select 21th ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR));

    memory.write(0x6000, 0); // Enabla 1 RAM banks mode
    memory.write(0x2000, 66); // Select 66th ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR));

    memory.write(0x2000, 88); // Select 88th ROM bank
    println!("{:x}", memory.read(ROM_SWITCHABLE_ADDR + 3));
}
