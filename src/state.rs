use super::*;

/*
 * State helps orchestrating whole system. It allows implementig features such as VRAM/OAM write restrictions
 * based on present GPU mode.
 */
pub struct State<T: BankController> {
    pub gpu: GPU,
    pub mmu: MMU<T>,
}

impl <T: BankController>State<T> {
    pub fn new(mmu: MMU<T>, gpu: GPU) -> Self {
        Self { mmu: mmu, gpu: gpu }
    }

    pub fn safe_write(&mut self, addr: Addr, value: Byte) {
        if self.is_addr_allowed(addr) { self.mmu.write(addr, value); }
    }

    pub fn safe_read(&mut self, addr: Addr) -> Byte {
        if self.is_addr_allowed(addr) { return self.mmu.read(addr) }
        0xFF
    }

    fn is_addr_allowed(&self, addr: Addr) -> bool {
        let is_vram = addr >= VRAM_ADDR && addr < VRAM_ADDR + VRAM_SIZE as Addr;
        let is_oam = addr >= OAM_ADDR && addr < OAM_ADDR + OAM_SIZE as Addr;

        if self.gpu.MODE == GPUMode::LCD_TRANSFER && is_vram { return false }
        if self.gpu.MODE == GPUMode::OAM_SEARCH && (is_oam || is_vram) { return false }
        true
    }
}