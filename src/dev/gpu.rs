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
 * OAM: 20, LCD: 43, HBLANK: 51. I assume LCD=160 so I had to change other values accrodingly to keep similar proportions.
 */
pub const OAM_SEARCH_CYCLES: u64 = 74;
pub const LCD_TRANSFER_CYCLES: u64 = 160;
pub const HBLANK_CYCLES: u64 = 190;
pub const SCANLINE_CYCLES: u64 = OAM_SEARCH_CYCLES + LCD_TRANSFER_CYCLES + HBLANK_CYCLES;
pub const VBLANK_CYCLES: u64 = SCANLINE_CYCLES * VBLANK_HEIGHT as u64;
pub const FRAME_CYCLES: u64 = SCANLINE_CYCLES * (SCREEN_HEIGHT + VBLANK_HEIGHT) as u64;

pub const TILE_MAP_1: u16 = 0x9800;
pub const TILE_MAP_2: u16 = 0x9C00;
pub const TILE_BLOCK_1: u16 = 0x8000;
pub const TILE_BLOCK_2: u16 = 0x9000;
pub const TILE_SIZE: u16 = 16;

pub type Color = (u8, u8, u8);
pub const WHITE: Color = (255, 255, 255);
pub const LIGHT_GRAY: Color = (110, 123, 138);
pub const DARK_GRAY: Color = (91, 97, 105);
pub const BLACK: Color = (0, 0, 0);

fn get_color(num: u8) -> Color {
    match num {
        0 => WHITE,
        1 => LIGHT_GRAY,
        2 => DARK_GRAY,
        3 => BLACK,
        _ => panic!("Invalid color {}. Only 0, 1, 2, 3 are valid colors.", num),
    }
}

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
    pub framebuff: Vec<Color>,
    
    // LCDC
    /* (0=Off, 1=On) */
    pub LCD_DISPLAY_ENABLE: bool,
    /* (0=9800-9BFF, 1=9C00-9FFF) */
    pub WINDOW_TILE_MAP: bool,
    /* (0=Off, 1=On) */
    pub WINDOW_ENABLED: bool,
    /* (0=8800-97FF, 1=8000-8FFF) For sprites it's always 8000-8FFF */
    pub TILE_ADDRESSING: bool,
    /* (0=9800-9BFF, 1=9C00-9FFF) */
    pub BG_TILE_MAP: bool,
    /* (0=8x8, 1=8x16) */
    pub SPRITE_SIZE: bool,
    /* 0=Off, 1=On) */
    pub SPRITE_ENABLED: bool,
    /* (0=Off, 1=On) */
    pub DISPLAY_PRIORITY: bool,

    // STAT
    pub COINCIDENCE_INTERRUPT_ENABLE: bool,
    pub MODE_2_OAM_INTERRUPT_ENABLE: bool,
    pub MODE_1_VBLANK_INTERRUPT_ENABLE: bool,
    pub MODE_0_HBLANK_INTERRUPT_ENABLE: bool,
    pub COINCIDENCE_FLAG: bool, // LY == LYC
    pub MODE: GPUMode,

    // BGP
    pub BG_COLOR_3_SHADE: u8,
    pub BG_COLOR_2_SHADE: u8,
    pub BG_COLOR_1_SHADE: u8,
    pub BG_COLOR_0_SHADE: u8,
    // OBP0
    pub OBP0_COLOR_3_SHADE: u8,
    pub OBP0_COLOR_2_SHADE: u8,
    pub OBP0_COLOR_1_SHADE: u8,
    pub OBP0_COLOR_0_SHADE: u8,
    // OBP1
    pub OBP1_COLOR_3_SHADE: u8,
    pub OBP1_COLOR_2_SHADE: u8,
    pub OBP1_COLOR_1_SHADE: u8,
    pub OBP1_COLOR_0_SHADE: u8,
}

impl GPU {
    pub fn new() -> Self {
        Self {
            framebuff: vec![WHITE; SCREEN_WIDTH*SCREEN_HEIGHT],
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

    // VBLANK_START
    fn on_vblank_start<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        self.MODE = GPUMode::VBLANK;
        self.ptr_y += 1;
        self.ptr_x = 0;

        self.flush_regs(mmu);
        GPU::vblank_int(mmu);
        if self.MODE_1_VBLANK_INTERRUPT_ENABLE {
            GPU::stat_int(mmu);
        }
    }

    // VBLANK END
    fn on_vblank_end<T: BankController>(&mut self, mmu: &mut MMU<T>){
        self.MODE = GPUMode::OAM_SEARCH;
        self.ptr_y = 0;
        self.ptr_x = 0;

        self.flush_regs(mmu);
        if self.MODE_2_OAM_INTERRUPT_ENABLE || (self.COINCIDENCE_INTERRUPT_ENABLE && self.COINCIDENCE_FLAG) {
            GPU::stat_int(mmu);
        }
    }

    // MID VBLANK
    // 144 <= LY <= 153 stands for VBLANK, so I assume that LY must be updated mid VBLANK.
    fn on_vblank_update<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        if self.cycle % SCANLINE_CYCLES != SCANLINE_CYCLES - 1 { return }
        self.ptr_y += 1;
        self.ptr_x = 0;

        self.flush_regs(mmu);
        if self.COINCIDENCE_INTERRUPT_ENABLE && self.COINCIDENCE_FLAG {
            GPU::stat_int(mmu);
        }
    }

    // HBLANK START
    fn on_hblank_start<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        self.MODE = GPUMode::HBLANK;

        self.flush_regs(mmu);
        if self.MODE_0_HBLANK_INTERRUPT_ENABLE {
            GPU::stat_int(mmu);
        }
    }

    // HBLANK END
    // Move ptr to next line, switch mode to OAM.
    fn on_hblank_end<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        self.MODE = GPUMode::OAM_SEARCH;
        self.ptr_y += 1;
        self.ptr_x = 0;
        
        self.flush_regs(mmu);
        if self.MODE_2_OAM_INTERRUPT_ENABLE {
            GPU::stat_int(mmu);
        }
    }

    // LCD TRANSFER START
    fn on_lcd_start<T: BankController>(&mut self, mmu: &mut MMU<T>){
        self.MODE = GPUMode::LCD_TRANSFER;

        self.flush_regs(mmu);
        if self.COINCIDENCE_INTERRUPT_ENABLE && self.COINCIDENCE_FLAG {
            GPU::stat_int(mmu);
        }
    }

    // MID LCD TRANSFER
    fn on_lcd_update<T: BankController>(&mut self, mmu: &mut MMU<T>) {        
        /*
         * BACKGROUND RENDER CODE
         */
        let scx = mmu.read(ioregs::SCX);
        let scy = mmu.read(ioregs::SCY);

        let x = (scx as u16 + self.ptr_x as u16) % 256;
        let y = (scy as u16 + self.ptr_y as u16) % 256;
        let x_tile = x/8;
        let y_tile = y/8;
        let off = (32*y_tile + x_tile) % 1024;
        let tile_map_base = if self.BG_TILE_MAP { TILE_MAP_2 } else { TILE_MAP_1 };
        let tile_num = mmu.read(tile_map_base + off);

        let tile_addr = 
          // 8000-8FFF unsigned addressing
          if self.TILE_ADDRESSING { TILE_BLOCK_1 + TILE_SIZE*tile_num as u16 }
          // 8800 signed addressing
          else if tile_num < 0x80 { TILE_BLOCK_2 + TILE_SIZE*tile_num as u16}
          else { TILE_BLOCK_2 - TILE_SIZE*(tile_num - 0x80) as u16};

        // Load tile data
        let tile: Vec<u8> = (0..TILE_SIZE).map(|i| mmu.read(tile_addr + i)).collect();
        let byte_x = x - x_tile*8;
        let byte_y = (y - y_tile*8) as usize;
        let (b1, b2) = (tile[2*byte_y], tile[2*byte_y+1]);
        let color = match (b2 & (0x80 >> byte_x) != 0, b1 & (0x80 >> byte_x) != 0) {
            (true, true) => 3,
            (true, false) => 2,
            (false, true) => 1,
            (false, false) => 0,
        };
        
        self.framebuff[self.ptr_y as usize * SCREEN_WIDTH + self.ptr_x as usize] = self.bg_color(color);

        self.ptr_x += 1;
    }

    pub fn step<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        // Re-read registers for updated config
        self.reread_regs(mmu);

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

        self.flush_regs(mmu);
        self.cycle = (self.cycle + 1) % FRAME_CYCLES;
    }

    pub fn reread_regs<T: BankController>(&mut self,mmu: &mut MMU<T>) {
        self.COINCIDENCE_FLAG = self.ptr_y == mmu.read(ioregs::LYC);

        self.lcdc(mmu.read(ioregs::LCDC));
        self.stat(mmu.read(ioregs::STAT));
        self.bgp(mmu.read(ioregs::BGP));
        self.obp0(mmu.read(ioregs::OBP_0));
        self.obp1(mmu.read(ioregs::OBP_1));
    }

    pub fn flush_regs<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        mmu.write(ioregs::LY, self.ptr_y);
        self.COINCIDENCE_FLAG = self.ptr_y == mmu.read(ioregs::LYC);
        //println!("LY: {}, LYC: {}, FLG: {}", self.ptr_y, mmu.read(ioregs::LYC), self.COINCIDENCE_FLAG);
        mmu.write(ioregs::LCDC, self.lcdc_new());
        mmu.write(ioregs::STAT, self.stat_new());
        mmu.write(ioregs::BGP, self.bgp_new());
        mmu.write(ioregs::OBP_0, self.obp0_new());
        mmu.write(ioregs::OBP_1, self.obp1_new());
    }

    fn lcdc(&mut self, byte: u8) {
        self.LCD_DISPLAY_ENABLE = (byte & 0x80) != 0;
        self.WINDOW_TILE_MAP    = (byte & 0x40) != 0;
        self.WINDOW_ENABLED     = (byte & 0x20) != 0;
        self.TILE_ADDRESSING    = (byte & 0x10) != 0;
        self.BG_TILE_MAP        = (byte & 0x8)  != 0;
        self.SPRITE_SIZE        = (byte & 0x4)  != 0;
        self.SPRITE_ENABLED     = (byte & 0x2)  != 0;
        self.DISPLAY_PRIORITY   = (byte & 0x1)  != 0;
    }

    fn lcdc_new(&self) -> u8 {
        let mut lcdc = 0u8;
        lcdc |= (self.LCD_DISPLAY_ENABLE as u8) << 7;
        lcdc |= (self.WINDOW_TILE_MAP as u8)    << 6;
        lcdc |= (self.WINDOW_ENABLED as u8)     << 5;
        lcdc |= (self.TILE_ADDRESSING as u8)    << 4;
        lcdc |= (self.BG_TILE_MAP as u8)        << 3;
        lcdc |= (self.SPRITE_SIZE as u8)        << 2;
        lcdc |= (self.SPRITE_ENABLED as u8)     << 1;
        lcdc |= (self.DISPLAY_PRIORITY as u8)   << 0;
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

    fn bgp(&mut self, byte: u8) {
        self.BG_COLOR_0_SHADE = (byte >> 0) & 0x03;
        self.BG_COLOR_1_SHADE = (byte >> 2) & 0x03;
        self.BG_COLOR_2_SHADE = (byte >> 4) & 0x03;
        self.BG_COLOR_3_SHADE = (byte >> 6) & 0x03;
    }

    fn bgp_new(&self) -> u8 {
        let mut bgp = 0u8;
        bgp |= (self.BG_COLOR_0_SHADE & 0x03) << 0;
        bgp |= (self.BG_COLOR_1_SHADE & 0x03) << 2;
        bgp |= (self.BG_COLOR_2_SHADE & 0x03) << 4;
        bgp |= (self.BG_COLOR_3_SHADE & 0x03) << 6;
        bgp
    }

    fn obp0(&mut self, byte: u8) {
        self.OBP0_COLOR_0_SHADE = (byte >> 0) & 0x03;
        self.OBP0_COLOR_1_SHADE = (byte >> 2) & 0x03;
        self.OBP0_COLOR_2_SHADE = (byte >> 4) & 0x03;
        self.OBP0_COLOR_3_SHADE = (byte >> 6) & 0x03;
    }

    fn obp0_new(&self) -> u8 {
        let mut obp0 = 0u8;
        obp0 |= (self.OBP0_COLOR_0_SHADE & 0x03) << 0;
        obp0 |= (self.OBP0_COLOR_1_SHADE & 0x03) << 2;
        obp0 |= (self.OBP0_COLOR_2_SHADE & 0x03) << 4;
        obp0 |= (self.OBP0_COLOR_3_SHADE & 0x03) << 6;
        obp0
    }

    fn obp1(&mut self, byte: u8) {
        self.OBP1_COLOR_0_SHADE = (byte >> 0) & 0x03;
        self.OBP1_COLOR_1_SHADE = (byte >> 2) & 0x03;
        self.OBP1_COLOR_2_SHADE = (byte >> 4) & 0x03;
        self.OBP1_COLOR_3_SHADE = (byte >> 6) & 0x03;
    }

    fn obp1_new(&self) -> u8 {
        let mut obp1 = 0u8;
        obp1 |= (self.OBP1_COLOR_0_SHADE & 0x03) << 0;
        obp1 |= (self.OBP1_COLOR_1_SHADE & 0x03) << 2;
        obp1 |= (self.OBP1_COLOR_2_SHADE & 0x03) << 4;
        obp1 |= (self.OBP1_COLOR_3_SHADE & 0x03) << 6;
        obp1
    }
    
    /*
     * Color translations based on current flags.
     */
    pub fn bg_color(&self, color: u8) -> Color {
        get_color(match color { 0 => self.BG_COLOR_0_SHADE, 1 => self.BG_COLOR_1_SHADE, 2 => self.BG_COLOR_2_SHADE, 3 => self.BG_COLOR_3_SHADE, _ => 0xFF })
    }

    pub fn obp0_color(&self, color: u8) -> Color {
        get_color(match color { 0 => self.OBP0_COLOR_0_SHADE, 1 => self.OBP0_COLOR_1_SHADE, 2 => self.OBP0_COLOR_2_SHADE, 3 => self.OBP0_COLOR_3_SHADE, _ => 0x80 })
    }

    pub fn obp1_color(&self, color: u8) -> Color {
        get_color(match color { 0 => self.OBP1_COLOR_0_SHADE, 1 => self.OBP1_COLOR_1_SHADE, 2 => self.OBP1_COLOR_2_SHADE, 3 => self.OBP1_COLOR_3_SHADE, _ => 0x40 })
    }
}