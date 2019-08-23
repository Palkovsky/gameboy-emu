use super::*;

/*
 * State helps orchestrating whole system. It allows implementig features such as VRAM/OAM write restrictions
 * based on present GPU mode. State allows triggering certain events after writing to specific IO registers, 
 * ex. write to LYC should trigger coincidence flag check in GPU.
 */
pub struct State<T: BankController> {
    pub gpu: GPU,
    pub timer: Timer,
    pub mmu: MMU<T>,
}

impl <T: BankController>State<T> {
    pub fn new(mapper: T) -> Self {
        let mut mmu = MMU::new(mapper);
        let gpu = GPU::new(&mut mmu);
        let timer = Timer::new();     
        Self { mmu: mmu, gpu: gpu, timer: timer }
    }

    pub fn safe_write(&mut self, addr: Addr, value: Byte) {
        if !self.is_addr_allowed(addr) { 
            println!("Tried writing to restricted memory at 0x{:x}", addr);  
        }

        self.mmu.write(addr, value);
        match addr {
            // LYC=LY flag should be updated constantly
            LYC => self.gpu.update(&mut self.mmu),
            // Write to DIV resets it to 0
            DIV => { 
                self.mmu.write(addr, 0); 
                self.timer.reset_internal_div();
            },
            TIMA => self.timer.reset_internal_tima(),
            _ => {},
        }
    }

    pub fn safe_read(&mut self, addr: Addr) -> Byte {
        if !self.is_addr_allowed(addr) { 
            println!("Tried reading from restricted memory at 0x{:x}", addr);  
            return 0xFF
        }
        
        self.mmu.read(addr)
    }

    fn is_addr_allowed(&mut self, addr: Addr) -> bool {
        let is_vram = addr >= VRAM_ADDR && addr < VRAM_ADDR + VRAM_SIZE as Addr;
        let is_oam = addr >= OAM_ADDR && addr < OAM_ADDR + OAM_SIZE as Addr;

        if GPU::MODE(&mut self.mmu) == GPUMode::LCD_TRANSFER && is_vram { return false }
        if GPU::MODE(&mut self.mmu) == GPUMode::OAM_SEARCH && (is_oam || is_vram) { return false }
        true
    }
}