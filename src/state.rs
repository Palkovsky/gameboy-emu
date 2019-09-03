use super::*;

/* CPU cycles per frame */
pub const CPU_CYCLES_PER_FRAME: u64 = (1<<20)/60;

/*
 * Runtime is used to connect CPU with everything stored in State(memory, IO devices).
 * I created it, cuz borrow checker yelld at me for doing something like this: self.cpu.step(self) // multiple mutable borrow
 */
pub struct Runtime<T: BankController> {
    pub cpu: CPU,
    pub state: State<T>,

    cpu_cycles: u64,
    gpu_cycles: u64,
    apu_cycles: u64,
    timer_cycles: u64,
}

impl <T: BankController>Runtime<T> {
    pub fn new(mapper: T) -> Self {
        let state = State::new(mapper);
        let cpu = CPU::new();
        Self { 
            cpu: cpu, state: state,
            cpu_cycles: 0,
            gpu_cycles: 0, 
            apu_cycles: 0,
            timer_cycles: 0,
        }
    }

    // Execute next instruction, handle interrupts and let other devices catchup.
    pub fn step(&mut self) {
        self.cpu_cycles += self.cpu.interrupts(&mut self.state);   
        self.cpu_cycles += self.cpu.step(&mut self.state);
        self.state.joypad.step(&mut self.state.mmu);
        if self.state.dma.active() {
            self.state.dma.step(&mut self.state.mmu);
        }   
        self.gpu_cycles = Runtime::catchup(&mut self.state.mmu, &mut self.state.gpu, self.cpu_cycles, self.gpu_cycles);
        self.timer_cycles = Runtime::catchup(&mut self.state.mmu, &mut self.state.timer, self.cpu_cycles, self.timer_cycles);
        self.apu_cycles = Runtime::catchup(&mut self.state.mmu, &mut self.state.apu, self.cpu_cycles, self.apu_cycles);
    }

    pub fn cpu_cycles(&self) -> u64 { self.cpu_cycles }
    pub fn reset_cycles(&mut self) {
        self.cpu_cycles = 0;
        self.gpu_cycles = 0;
        self.apu_cycles = 0;
        self.timer_cycles = 0;
    }

    fn catchup(mmu: &mut MMU<T>, dev: &mut impl Clocked<T>, cpu_clk: u64, dev_clk: u64) -> u64 {
        let mut next = dev.next_time(mmu);
        let mut dev_new = dev_clk;
        while dev_new + next <= cpu_clk {
            dev_new += next;
            dev.step(mmu);
            next = dev.next_time(mmu);
        }
        dev_new
    }
}

/*
 * State is middleware between CPU<->Memory/IO. It offers CPU safe interface for writng/reading memory which helps achieving 
 * certain constrains that couldn't be done inside single device.
 * For example: updatde coincidence flag when LYC changes or disallow VRAM/OAM access when GPU is rendering.
 */
pub struct State<T: BankController> {
    pub gpu: GPU,
    pub apu: APU,
    pub timer: Timer,
    pub dma: DMA,
    pub joypad: Joypad,
    pub mmu: MMU<T>,
}

impl <T: BankController>State<T> {
    pub fn new(mapper: T) -> Self {
        let mut mmu = MMU::new(mapper);
        let gpu = GPU::new(&mut mmu);
        let apu = APU::new(&mut mmu);
        let timer = Timer::new();
        let dma = DMA::new();
        let joypad = Joypad::new();     
        Self { mmu: mmu, gpu: gpu, apu: apu, timer: timer, dma: dma, joypad: joypad }
    }

    pub fn safe_write(&mut self, addr: Addr, value: Byte) {
        self.mmu.write(addr, value);
        match addr {
            // LYC=LY flag should be updated constantly
            LYC => self.gpu.update(&mut self.mmu),
            // NR_14 => self.apu.chan1_reset(&mut self.mmu);,
            //NR_21 | NR_22 | NR_23 | NR_24         => self.apu.chan2_reset(&mut self.mmu),
            //NR_30 | NR_31 | NR_32 | NR_33 | NR_34 => self.apu.chan3_reset(&mut self.mmu),
            //NR_41 | NR_42 | NR_43 | NR_44         => self.apu.chan2_reset(&mut self.mmu),
            // Write to DIV resets it to 0
            DIV => { 
                self.mmu.write(addr, 0); 
                self.timer.reset_internal_div();
            },
            TIMA => self.timer.reset_internal_tima(),
            // Write to DMA register starts DMA transfer
            ioregs::DMA => self.dma.start(),
            _ => {},
        }
    }

    pub fn write_word(&mut self, addr: Addr, word: Word) {
        self.safe_write(addr, (word & 0xFF) as u8);
        self.safe_write(addr+1, (word >> 8) as u8);
    }

    pub fn safe_read(&mut self, addr: Addr) -> Byte {
        self.mmu.read(addr)
    }

    pub fn read_word(&mut self, addr: Addr) -> Word {
        self.safe_read(addr) as u16 + ((self.safe_read(addr+1) as u16) << 8)
    }

    fn is_addr_allowed(&mut self, addr: Addr) -> bool {
        let is_vram = addr >= VRAM_ADDR && addr < VRAM_ADDR + VRAM_SIZE as Addr;
        let is_oam = addr >= OAM_ADDR && addr < OAM_ADDR + OAM_SIZE as Addr;

        if GPU::MODE(&mut self.mmu) == GPUMode::LCD_TRANSFER && is_vram { return false }
        if GPU::MODE(&mut self.mmu) == GPUMode::OAM_SEARCH && (is_oam || is_vram) { return false }
        true
    }
}