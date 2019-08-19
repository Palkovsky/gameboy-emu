use super::*;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;
pub const VBLANK_HEIGHT: usize = 10;

/*
 * MODE 0 - HBLANK
 * MODE 1 - VBLANK
 * MODE 2 - OAM SEARCH
 * MODE 3 - LCD TRANSFER
 * Below values keep orginal ratio of phases, but provide per-dot granularity.
 * OAM: 20, LCD: 43, HBLANK: 51. I assume LCD=160 and so I had to change other values accrodingly.
 */
pub const OAM_SEARCH_CYCLES: u64 = 74;
pub const LCD_TRANSFER_CYCLES: u64 = 160;
pub const HBLANK_CYCLES: u64 = 190;
pub const SCANLINE_CYCLES: u64 = OAM_SEARCH_CYCLES + LCD_TRANSFER_CYCLES + HBLANK_CYCLES;
pub const VBLANK_CYCLES: u64 = SCANLINE_CYCLES * VBLANK_HEIGHT as u64;
pub const FRAME_CYCLES: u64 = SCANLINE_CYCLES * (SCREEN_HEIGHT + VBLANK_HEIGHT) as u64;

#[derive(Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub enum GPUMode {
    HBLANK, VBLANK, OAM_SEARCH, LCD_TRANSFER,
}
impl Default for GPUMode {
    fn default() -> Self { GPUMode::OAM_SEARCH }
}

#[derive(Default)]
#[allow(non_snake_case)]
pub struct GPU {
    ptr_x: u8,
    ptr_y: u8,
    cycle: u64,
    first_step: bool,
    framebuff: Vec<u8>,
    
    // LCDC
    pub LCD_DISPLAY_ENABLE: bool,
    pub WINDOW_TILE_MAP_SELECT: bool,
    pub WINDOW_DISPLAY_ENABLE: bool,
    pub BG_WINDOW_TILE_DATA_SELECT: bool,
    pub BG_TILE_MAP_DISPLAY_SELECT: bool,
    pub SPRITE_SIZE: bool,
    pub SPRITE_DISPLAY_ENABLE: bool,
    pub DISPLAY_PRIORITY: bool,

    // STAT
    pub COINCIDENCE_INTERRUPT_ENABLE: bool,
    pub MODE_2_OAM_INTERRUPT_ENABLE: bool,
    pub MODE_1_VBLANK_INTERRUPT_ENABLE: bool,
    pub MODE_0_HBLANK_INTERRUPT_ENABLE: bool,
    pub COINCIDENCE_FLAG: bool, // LY == LYC
    pub MODE: GPUMode,
}

impl GPU {
    pub fn new() -> Self {
        Self {
            framebuff: vec![0; SCREEN_WIDTH*SCREEN_HEIGHT],
            first_step: true,
            ..Default::default()
        }
    }

    fn vblank_int<T: BankController>(mmu: &mut MMU<T>) {
        let iflg = mmu.read(ioregs::IF);
        mmu.write(ioregs::IF, iflg | 1);
    }

    fn stat_int<T: BankController>(mmu: &mut MMU<T>) {
        let iflg = mmu.read(ioregs::IF);
        mmu.write(ioregs::IF, iflg | 2);
    }

    fn update_ly<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        // Update LY, set/unset coincidence flag
        mmu.write(ioregs::LY, self.ptr_y);
        self.COINCIDENCE_FLAG = mmu.read(ioregs::LY) == mmu.read(ioregs::LYC);
    
        // If coincidence interrupt set -> make STAT interrupt
        if self.COINCIDENCE_FLAG && self.COINCIDENCE_INTERRUPT_ENABLE {
            GPU::stat_int(mmu);
        }
    }

    // VBLANK_START
    fn on_vblank_start<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        self.MODE = GPUMode::VBLANK;
        self.ptr_y += 1;
        self.ptr_x = 0;
        self.update_ly(mmu);
        GPU::vblank_int(mmu);
    }

    // VBLANK END
    fn on_vblank_end<T: BankController>(&mut self, mmu: &mut MMU<T>){
        self.MODE = GPUMode::OAM_SEARCH;
        self.ptr_y = 0;
        self.ptr_x = 0;
        self.update_ly(mmu);
    }

    // MID VBLANK
    fn on_vblank_update<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        if self.cycle % SCANLINE_CYCLES != SCANLINE_CYCLES - 1 { return }
        self.ptr_y += 1;
        self.ptr_x = 0;
        self.update_ly(mmu); 
    }

    // HBLANK START
    fn on_hblank_start<T: BankController>(&mut self, _: &mut MMU<T>) {
        self.MODE = GPUMode::HBLANK;
    }

    // OAM START
    fn on_hblank_end<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        self.MODE = GPUMode::OAM_SEARCH;
        self.ptr_y += 1;
        self.ptr_x = 0;
        self.update_ly(mmu);
    }

    // LCD TRANSFER START
    fn on_lcd_start<T: BankController>(&mut self, _: &mut MMU<T>){
        self.MODE = GPUMode::LCD_TRANSFER;
    }

    // MID LCD TRANSFER
    fn on_lcd_update<T: BankController>(&mut self, _: &mut MMU<T>) {
        self.ptr_x += 1;
    }

    pub fn step<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        // Re-read registers for updated config
        self.lcdc(mmu.read(ioregs::LCDC));
        self.stat(mmu.read(ioregs::STAT));

        // Update IO regs if this is first call of step()
        if self.first_step { 
            self.update_ly(mmu);
            self.first_step = false;
        }

        // Where are we on current scanline?
        let line_cycle = self.cycle % SCANLINE_CYCLES;

        // Starting VBLANK
        if self.MODE == GPUMode::HBLANK && self.ptr_y == SCREEN_HEIGHT as u8 - 1 && line_cycle == SCANLINE_CYCLES - 1 {
            self.on_vblank_start(mmu);
        }
        // End of VBLANK
        else if self.MODE == GPUMode::VBLANK && self.cycle == FRAME_CYCLES - 1 {
            self.on_vblank_end(mmu);
        }
        // Starting HBLANK
        else if self.MODE == GPUMode::LCD_TRANSFER && self.ptr_x == SCREEN_WIDTH as u8 - 1 {
            self.on_hblank_start(mmu);
        }
        // Ending HBLANK
        else if self.MODE == GPUMode::HBLANK && line_cycle == SCANLINE_CYCLES - 1 {
            self.on_hblank_end(mmu);
        } 
        // Ending OAM_SEARCH
        else if self.MODE == GPUMode::OAM_SEARCH && line_cycle == OAM_SEARCH_CYCLES - 1 {
            self.on_lcd_start(mmu);
        } 
        // During VBLANK
        else if self.MODE == GPUMode::VBLANK {
            self.on_vblank_update(mmu);
        }
        // During LCD_TRANSFER
        else if self.MODE == GPUMode::LCD_TRANSFER {
            self.on_lcd_update(mmu);
        } 

        mmu.write(ioregs::LCDC, self.lcdc_new());
        mmu.write(ioregs::STAT, self.stat_new());
        self.cycle = (self.cycle + 1) % FRAME_CYCLES;
    }

    fn lcdc(&mut self, byte: u8) {
        self.LCD_DISPLAY_ENABLE         = (byte & 0x80) != 0;
        self.WINDOW_TILE_MAP_SELECT     = (byte & 0x40) != 0;
        self.WINDOW_DISPLAY_ENABLE      = (byte & 0x20) != 0;
        self.BG_WINDOW_TILE_DATA_SELECT = (byte & 0x10) != 0;
        self.BG_TILE_MAP_DISPLAY_SELECT = (byte & 0x8)  != 0;
        self.SPRITE_SIZE                = (byte & 0x4)  != 0;
        self.SPRITE_DISPLAY_ENABLE      = (byte & 0x2)  != 0;
        self.DISPLAY_PRIORITY           = (byte & 0x1)  != 0;
    }

    fn lcdc_new(&self) -> u8 {
        let mut lcdc = 0u8;
        lcdc |= (self.LCD_DISPLAY_ENABLE as u8)         << 7;
        lcdc |= (self.WINDOW_TILE_MAP_SELECT as u8)     << 6;
        lcdc |= (self.WINDOW_DISPLAY_ENABLE as u8)      << 5;
        lcdc |= (self.BG_WINDOW_TILE_DATA_SELECT as u8) << 4;
        lcdc |= (self.BG_TILE_MAP_DISPLAY_SELECT as u8) << 3;
        lcdc |= (self.SPRITE_SIZE as u8)                << 2;
        lcdc |= (self.SPRITE_DISPLAY_ENABLE as u8)      << 1;
        lcdc |= (self.DISPLAY_PRIORITY as u8)           << 0;
        lcdc 
    }

    fn stat(&mut self, byte: u8) {
        self.COINCIDENCE_INTERRUPT_ENABLE   = (byte & 0x40) != 0;
        self.MODE_2_OAM_INTERRUPT_ENABLE    = (byte & 0x20) != 0;
        self.MODE_1_VBLANK_INTERRUPT_ENABLE = (byte & 0x10) != 0;
        self.MODE_0_HBLANK_INTERRUPT_ENABLE = (byte & 0x8)  != 0;
    }

    fn stat_new(&self) -> u8 {
        let mut stat = 0u8;
        stat |= (self.COINCIDENCE_INTERRUPT_ENABLE as u8)   << 6;
        stat |= (self.MODE_2_OAM_INTERRUPT_ENABLE as u8)    << 5;
        stat |= (self.MODE_1_VBLANK_INTERRUPT_ENABLE as u8) << 4;
        stat |= (self.MODE_0_HBLANK_INTERRUPT_ENABLE as u8) << 3;
        stat |= (self.COINCIDENCE_FLAG as u8)               << 2;
        stat |= match self.MODE {
            GPUMode::HBLANK => 0,
            GPUMode::VBLANK => 1,
            GPUMode::OAM_SEARCH => 2,
            GPUMode::LCD_TRANSFER => 3,
        };
        stat
    }
}