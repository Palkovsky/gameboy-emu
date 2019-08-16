use super::*;

/*
 * Simplest MBC - no switching needed. Only 32KB of ROM available and one 8KB bank of RAM.
 */
const ROM_ONLY_SIZE: usize = 1 << 15;

pub struct RomOnly {
    pub rom: Vec<Byte>,
}

impl RomOnly {
    pub fn new(rom: Vec<Byte>) -> Self { 
        let mut mbc = Self {  rom: vec![0; ROM_ONLY_SIZE] };
        if rom.len() > mbc.rom.len() { panic!("ROM too big for RomOnly"); }
        for (i, byte) in rom.into_iter().enumerate() { mbc.rom[i] = byte; }
        mbc
    }
}

impl BankController for RomOnly {
    fn get_addr_type(&self, _: Addr) -> AddrType { 
        AddrType::Write 
    }    

    fn on_status(&mut self, _: Addr, _: Byte) {}

    fn get_base_rom(&mut self) -> Option<MutMem> { 
        Some(&mut self.rom[..ROM_BANK_SIZE])
    }
    
    fn get_switchable_rom(&mut self) -> Option<MutMem> { 
        Some(&mut self.rom[ROM_BANK_SIZE..ROM_BANK_SIZE*2])
    }

    fn get_switchable_ram(&mut self) -> Option<MutMem> { None }
}