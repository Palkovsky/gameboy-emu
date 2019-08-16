use super::*;

/*
 * Simplest MBC - no switching needed. This implementation assumes that switchable RAM
 * bank(0xA000-0xBFFF) is available.
 */
const ROM_ONLY_SIZE: usize = 1 << 15;

pub struct RomOnly {
    rom_banks: Vec<Byte>,
}

impl RomOnly {
    pub fn new(rom: Vec<Byte>) -> Self { 
        let mut mbc = Self { rom_banks: vec![0; ROM_ONLY_SIZE] };
        if rom.len() > mbc.rom_banks.len() { panic!("ROM too big for RomOnly"); }
        for (i, byte) in rom.into_iter().enumerate() { mbc.rom_banks[i] = byte; }
        mbc
    }
}

impl BankController for RomOnly {
    fn get_addr_type(&self, _: Addr) -> AddrType { AddrType::Write }    
    fn on_status(&mut self, _: Addr, _: Byte) {}
    fn get_base_rom(&mut self) -> Option<MutMem> { Some(&mut self.rom_banks[..ROM_BANK_SIZE]) }
    fn get_switchable_rom(&mut self) -> Option<MutMem> { None }
    fn get_base_ram(&mut self) -> Option<MutMem> { None }
    fn get_switchable_ram(&mut self) -> Option<MutMem> { Some(&mut self.rom_banks[ROM_BANK_SIZE..ROM_BANK_SIZE*2])}
}