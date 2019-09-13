use super::*;

const RAM_BANKS: usize = 4;
const ROM_BANKS: usize = 128;
pub const RAM_MODE: u8 = 1;
pub const ROM_MODE: u8 = 0;

pub struct MBC1 {
    pub ram: Vec<Byte>,
    pub rom: Vec<Byte>,
    pub ram_enabled: bool,
    pub banking_mode: u8,
    idx: u8,
}

impl MBC1 {
    pub fn new(rom: Vec<Byte>) -> Self {
        let mut mbc = Self {
            ram: vec![0; RAM_BANK_SIZE*RAM_BANKS],
            rom: vec![0; ROM_BANK_SIZE*ROM_BANKS],
            ram_enabled: false,
            banking_mode: ROM_MODE,
            idx: 1,
        };
        if rom.len() > mbc.rom.len() { panic!("ROM too big for MBC1"); }
        for (i, byte) in rom.into_iter().enumerate() { mbc.rom[i] = byte; }
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
            if value == 0x00 {
                self.ram_enabled = false;
            } else if value == 0x0A {
                self.ram_enabled = true;
            }
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
            // if self.idx & 0x1F == 0 { self.idx += 1; } // If 5 lower bits are zeros => change to 1
        }
        // 0x6000 - 0x8000 -> Banking Mode(RAM/ROM)
        // For ROM(8KB RAM, 2MB ROM): XXXXXXX1, for RAM(32KB RAM, 512KB ROM): XXXXXXX0
        if addr >= 0x6000 && addr < 0x8000 {
            self.banking_mode = value & 1;
        }
    }

    fn get_base_rom(&mut self) -> Option<MutMem> { Some(&mut self.rom[..ROM_BANK_SIZE]) }

    fn get_switchable_rom(&mut self) -> Option<MutMem> {
        let mask = if self.banking_mode == ROM_MODE {
            0x7F
        } else {
            0x1F
        };
        let rom_idx = self.idx & mask;
        let start = (rom_idx as usize) * ROM_BANK_SIZE;
        let end = start + ROM_BANK_SIZE;
        Some(&mut self.rom[start..end])
    }

    fn get_switchable_ram(&mut self) -> Option<MutMem> {
        println!("MODE: {}, ENABLED: {}", self.banking_mode, self.ram_enabled);
        if !self.ram_enabled { return None }

        let mask = if self.banking_mode == RAM_MODE {
            0b01100000
        } else {
            0
        } >> 5;

        let ram_idx = self.idx & mask;
        let start = (ram_idx as usize) * RAM_BANK_SIZE;
        let end = start + RAM_BANK_SIZE;
        Some(&mut self.ram[start..end])
    }
}
