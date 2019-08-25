use super::*;

/*
 * Runtime is used to connect CPU with everything stored in State(memory, IO devices).
 * I created it, cuz borrow checker yelld at me for doing something like this: self.cpu.step(self) // multiple mutable borrow
 */
pub struct Runtime<T: BankController> {
    pub cpu: CPU,
    pub state: State<T>,
}

impl <T: BankController>Runtime<T> {
    pub fn new(mapper: T) -> Self {
        let mut state = State::new(mapper);
        let cpu = CPU::new();
        
        state.mmu.booting(true);
        
        Self { cpu: cpu, state: state }
    }

    pub fn step(&mut self) {
        // Detect end of boot sequence
        if self.state.mmu.is_booting() && self.cpu.PC.val() >= 0x100 {
            self.state.mmu.booting(false);
        }

        // Do next instruction cycle
        self.cpu.step(&mut self.state);
        self.cpu.interrupts(&mut self.state);
    }
}

/*
 * State is middleware between CPU<->Memory/IO. It offers CPU safe interface for writng/reading memory which helps achieving 
 * certain constrains that couldn't be done inside single device.
 * For example: updatde coincidence flag when LYC changes or disallow VRAM/OAM access when GPU is rendering.
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