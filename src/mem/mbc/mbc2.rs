use super::*;

/*
 * MBC2 doesn't support switchable RAM banks. It only has 512x4bit internal RAM.
 * Internal RAM is mapped to A000-A1FFF
 */

const RAM_SIZE: usize = 512;
const ROM_BANKS: usize = 16;

pub struct MBC2 {
    pub ram: Vec<Byte>,
    pub rom: Vec<Byte>,
    ram_enabled: bool,
    idx: u8,
}

impl MBC2 {
    pub fn new(rom: Vec<Byte>) -> Self {
        let mut mbc = Self {
            ram: vec![0; RAM_SIZE],
            rom: vec![0; ROM_BANK_SIZE*ROM_BANKS],
            ram_enabled: true, idx: 0,
        };
        if rom.len() > mbc.rom.len() { panic!("ROM too big for MBC2"); }
        for (i, byte) in rom.into_iter().enumerate() { mbc.rom[i] = byte; }
        mbc
    }
}

impl BankController for MBC2 {
    fn get_addr_type(&self, addr: Addr) -> AddrType {
        let intervals = [
            (0x0000, 0x1FFF),  // RAM enable
            (0x2000, 0x3FFF),  // ROM bank select
        ];
        for (start, end) in intervals.iter() {
            if addr >= *start && addr <= *end { return AddrType::Status }
        }
        AddrType::Write
    }   

    fn on_status(&mut self, addr: Addr, value: Byte) {
        // 0x0000 - 0x2000 -> RAM ON/OFF
        if addr & 0x1000 == 0 && addr < 0x2000 { 
            //println!("RAM ENABLED: {} -> {}", self.ram_enabled, value & 0xF == 0xA);
            self.ram_enabled = value & 0xF == 0xA;
        }

        // 0x2000 - 0x4000 -> ROM Select
        if addr & 0x0100 != 0 && addr >= 0x2000 && addr < 0x4000 {
            let idx = value & 0xF;
            //println!("ROM SELECT: {} -> {}", self.idx, idx);
            self.idx = idx;
        }
    }

    fn get_base_rom(&mut self) -> Option<MutMem> { 
        Some(&mut self.rom[..ROM_BANK_SIZE]) 
    }

    fn get_switchable_rom(&mut self) -> Option<MutMem> {
        let rom_idx = self.idx;
        let start = (rom_idx as usize) * ROM_BANK_SIZE;
        let end = start + ROM_BANK_SIZE;
        Some(&mut self.rom[start..end])
    }

    fn get_switchable_ram(&mut self) -> Option<MutMem> {
        if !self.ram_enabled { return None }

        // Make sure there are only 4bit numbers in RAM.
        for item in self.ram.iter_mut() { 
            *item &= 0xF;
        }

        Some(&mut self.ram[..])     }
}
