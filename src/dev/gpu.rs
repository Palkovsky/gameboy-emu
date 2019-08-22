use super::*;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;
pub const VBLANK_HEIGHT: usize = 10;

/*
 * MODE 0 - HBLANK
 * MODE 1 - VBLANK
 * MODE 2 - OAM SEARCH
 * MODE 3 - LCD TRANSFER
 * Below values are internal cycles. To convert to CPU cycles: x*43/160.
 * CPU CYCLES: OAM: 20, LCD: 43, HBLANK: 51. I assume LCD=160 so I had to change other values accrodingly to keep similar proportions.
 */
pub const OAM_SEARCH_CYCLES: u64 = 70;
pub const LCD_TRANSFER_CYCLES: u64 = 160;
pub const HBLANK_CYCLES: u64 = 188;
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
pub const TRANSPARENT: Color = (0, 21, 37); // It will be recognized by renderer as transparent

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
    lyc_interrupted: bool, // was window drawn in current line?
    pub framebuff: Vec<Color>,
}

impl GPU {
    pub fn new() -> Self {
        Self {
            framebuff: vec![WHITE; SCREEN_WIDTH*SCREEN_HEIGHT],
            lyc_interrupted: false,
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
        GPU::_MODE(mmu, GPUMode::VBLANK);
        self.ptr_y += 1;
        self.ptr_x = 0;

        self.update_ly(mmu);
        GPU::vblank_int(mmu);
        if GPU::MODE_1_VBLANK_INTERRUPT_ENABLE(mmu) {
            GPU::stat_int(mmu);
        }
    }

    // VBLANK END
    fn on_vblank_end<T: BankController>(&mut self, mmu: &mut MMU<T>){
        GPU::_MODE(mmu, GPUMode::OAM_SEARCH);
        self.ptr_y = 0;
        self.ptr_x = 0;
        self.lyc_interrupted = false;

        println!("VBLANK END");
        self.update_ly(mmu);
        if GPU::MODE_2_OAM_INTERRUPT_ENABLE(mmu) { GPU::stat_int(mmu); }
    }

    // MID VBLANK
    // 144 <= LY <= 153 stands for VBLANK, so I assume that LY must be updated mid VBLANK.
    fn on_vblank_update<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        if self.cycle % SCANLINE_CYCLES != SCANLINE_CYCLES - 1 { return }
        self.ptr_y += 1;
        self.ptr_x = 0;
        
        self.update_ly(mmu);
    }

    // HBLANK START
    fn on_hblank_start<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        GPU::_MODE(mmu, GPUMode::HBLANK);

        if GPU::MODE_0_HBLANK_INTERRUPT_ENABLE(mmu) { GPU::stat_int(mmu); }
    }

    // HBLANK END
    // Move ptr to next line, switch mode to OAM.
    fn on_hblank_end<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        GPU::_MODE(mmu, GPUMode::OAM_SEARCH);
        self.ptr_y += 1;
        self.ptr_x = 0;
        self.lyc_interrupted = false;
        self.update_ly(mmu);
        
        if GPU::MODE_2_OAM_INTERRUPT_ENABLE(mmu) { GPU::stat_int(mmu); }
    }

    // LCD TRANSFER START
    fn on_lcd_start<T: BankController>(&mut self, mmu: &mut MMU<T>){
        GPU::_MODE(mmu, GPUMode::LCD_TRANSFER);
    }

    // MID LCD TRANSFER
    // Currently it draws one pixel. In future it should draw whole single line and update clock accrodingly.
    fn on_lcd_update<T: BankController>(&mut self, mmu: &mut MMU<T>) {        
        let wx =  mmu.read(ioregs::WX);
        let wy =  mmu.read(ioregs::WY);
        let is_window = GPU::WINDOW_ENABLED(mmu) && self.ptr_x >= wx && self.ptr_y >= wy && wx >= 7 && wx <= 166 && wy <= 143;

        let (x, y, tile_map_base) = if is_window {
            (self.ptr_x as u16 - 7, self.ptr_y as u16, if GPU::WINDOW_TILE_MAP(mmu) { TILE_MAP_2 } else { TILE_MAP_1 })
        } else {
            let scx = mmu.read(ioregs::SCX);
            let scy = mmu.read(ioregs::SCY);
            ((scx as u16 + self.ptr_x as u16) % 256, (scy as u16 + self.ptr_y as u16) % 256, if GPU::BG_TILE_MAP(mmu) { TILE_MAP_2 } else { TILE_MAP_1 })
        };

        /* BACKGROUND/WINDOW RENDERING */
        let x_tile = x/8;
        let y_tile = y/8;
        let off = (32*y_tile + x_tile) % 1024;
        let tile_num = mmu.read(tile_map_base + off);

        let tile_addr = 
          // 8000-8FFF unsigned addressing
          if GPU::TILE_ADDRESSING(mmu) { TILE_BLOCK_1 + TILE_SIZE*tile_num as u16 }
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
        
        self.framebuff[self.ptr_y as usize * SCREEN_WIDTH + self.ptr_x as usize] = GPU::bg_color(mmu, color);
        self.ptr_x += 1;
    }

    pub fn step<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        if !GPU::LCD_DISPLAY_ENABLE(mmu) {
            return
        }

        self.update_ly(mmu);
        if !self.lyc_interrupted && GPU::COINCIDENCE_FLAG(mmu) && GPU::COINCIDENCE_INTERRUPT_ENABLE(mmu) {
            GPU::stat_int(mmu);
            self.lyc_interrupted = true;
        }

        // Where are we on current scanline?
        let line_cycle = self.cycle % SCANLINE_CYCLES;
        let mode = GPU::MODE(mmu);

        // Starting VBLANK
        if mode == GPUMode::HBLANK && self.ptr_y == SCREEN_HEIGHT as u8 - 1 && line_cycle == SCANLINE_CYCLES - 1 {
            self.on_vblank_start(mmu);
        }
        // End of VBLANK
        else if mode == GPUMode::VBLANK && self.cycle == FRAME_CYCLES - 1 {
            self.on_vblank_end(mmu);
        }
        // Starting HBLANK
        else if mode == GPUMode::LCD_TRANSFER && self.ptr_x == SCREEN_WIDTH as u8 - 1 {
            self.on_lcd_update(mmu);
            self.on_hblank_start(mmu);
        }
        // Ending HBLANK
        else if mode == GPUMode::HBLANK && line_cycle == SCANLINE_CYCLES - 1 {
            self.on_hblank_end(mmu);
        } 
        // Ending OAM_SEARCH
        else if mode == GPUMode::OAM_SEARCH && line_cycle == OAM_SEARCH_CYCLES - 1 {
            self.on_lcd_start(mmu);
        } 
        // During VBLANK
        else if mode == GPUMode::VBLANK {
            self.on_vblank_update(mmu);
        }
        // During LCD_TRANSFER
        else if mode == GPUMode::LCD_TRANSFER {
            self.on_lcd_update(mmu);
        } 

        self.cycle = (self.cycle + 1) % FRAME_CYCLES;
    }

    fn update_ly<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        let lyc = GPU::LYC(mmu);
        // println!("GPU | LYC {}, LINE {}", lyc, self.ptr_y);
        GPU::_LY(mmu, self.ptr_y);
        GPU::_COINCIDENCE_FLAG(mmu, self.ptr_y == lyc);
    }

    pub fn LY<T: BankController>(mmu: &mut MMU<T>) -> u8 { mmu.read(ioregs::LY) }
    pub fn LYC<T: BankController>(mmu: &mut MMU<T>) -> u8 { mmu.read(ioregs::LYC) }
    pub fn _LY<T: BankController>(mmu: &mut MMU<T>, val: u8) { mmu.write(ioregs::LY, val); }

    // LCDC GETTERS
    /* (0=Off, 1=On) */
    pub fn LCD_DISPLAY_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool { mmu.read_bit(ioregs::LCDC, 7) }
    /* (0=9800-9BFF, 1=9C00-9FFF) */
    pub fn WINDOW_TILE_MAP<T: BankController>(mmu: &mut MMU<T>) -> bool    { mmu.read_bit(ioregs::LCDC, 6) }
    /* (0=Off, 1=On) */
    pub fn WINDOW_ENABLED<T: BankController>(mmu: &mut MMU<T>) -> bool     { mmu.read_bit(ioregs::LCDC, 5) }
    /* (0=8800-97FF, 1=8000-8FFF) For sprites it's always 8000-8FFF */
    pub fn TILE_ADDRESSING<T: BankController>(mmu: &mut MMU<T>) -> bool    { mmu.read_bit(ioregs::LCDC, 4) }
    /* (0=9800-9BFF, 1=9C00-9FFF) */
    pub fn BG_TILE_MAP<T: BankController>(mmu: &mut MMU<T>) -> bool        { mmu.read_bit(ioregs::LCDC, 3) }
    /* (0=8x8, 1=8x16) */
    pub fn SPRITE_SIZE<T: BankController>(mmu: &mut MMU<T>) -> bool        { mmu.read_bit(ioregs::LCDC, 2) }
    /* 0=Off, 1=On) */
    pub fn SPRITE_ENABLED<T: BankController>(mmu: &mut MMU<T>) -> bool     { mmu.read_bit(ioregs::LCDC, 1) }
    /* (0=Off, 1=On) */
    pub fn DISPLAY_PRIORITY<T: BankController>(mmu: &mut MMU<T>) -> bool   { mmu.read_bit(ioregs::LCDC, 0) }

    // LCDC SETTERS
    pub fn _LCD_DISPLAY_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool) { mmu.set_bit(ioregs::LCDC, 7, flg) }
    pub fn _WINDOW_TILE_MAP<T: BankController>(mmu: &mut MMU<T>, flg: bool)    { mmu.set_bit(ioregs::LCDC, 6, flg) }
    pub fn _WINDOW_ENABLED<T: BankController>(mmu: &mut MMU<T>, flg: bool)     { mmu.set_bit(ioregs::LCDC, 5, flg) }
    pub fn _TILE_ADDRESSING<T: BankController>(mmu: &mut MMU<T>, flg: bool)    { mmu.set_bit(ioregs::LCDC, 4, flg) }
    pub fn _BG_TILE_MAP<T: BankController>(mmu: &mut MMU<T>, flg: bool)        { mmu.set_bit(ioregs::LCDC, 3, flg) }
    pub fn _SPRITE_SIZE<T: BankController>(mmu: &mut MMU<T>, flg: bool)        { mmu.set_bit(ioregs::LCDC, 2, flg) }
    pub fn _SPRITE_ENABLED<T: BankController>(mmu: &mut MMU<T>, flg: bool)     { mmu.set_bit(ioregs::LCDC, 1, flg) }
    pub fn _DISPLAY_PRIORITY<T: BankController>(mmu: &mut MMU<T>, flg: bool)   { mmu.set_bit(ioregs::LCDC, 0, flg) }

    // STAT GETTERS
    pub fn COINCIDENCE_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool   { mmu.read_bit(ioregs::STAT, 6) }
    pub fn MODE_2_OAM_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool    { mmu.read_bit(ioregs::STAT, 5) }
    pub fn MODE_1_VBLANK_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool { mmu.read_bit(ioregs::STAT, 4) }
    pub fn MODE_0_HBLANK_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool { mmu.read_bit(ioregs::STAT, 3) }
    pub fn COINCIDENCE_FLAG<T: BankController>(mmu: &mut MMU<T>) -> bool               { mmu.read_bit(ioregs::STAT, 2) }
    pub fn MODE<T: BankController>(mmu: &mut MMU<T>) -> GPUMode { 
        match mmu.read(ioregs::STAT) & 0x3 { 
            0 => GPUMode::HBLANK, 
            1 => GPUMode::VBLANK, 
            2 => GPUMode::OAM_SEARCH, 
            _ => GPUMode::LCD_TRANSFER 
        }
    }

    // STAT SETTERS
    pub fn _COINCIDENCE_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool)   { mmu.set_bit(ioregs::STAT, 6, flg) }
    pub fn _MODE_2_OAM_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool)    { mmu.set_bit(ioregs::STAT, 5, flg) }
    pub fn _MODE_1_VBLANK_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool) { mmu.set_bit(ioregs::STAT, 4, flg) }
    pub fn _MODE_0_HBLANK_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool) { mmu.set_bit(ioregs::STAT, 3, flg)}
    pub fn _COINCIDENCE_FLAG<T: BankController>(mmu: &mut MMU<T>, flg: bool)               { mmu.set_bit(ioregs::STAT, 2, flg) }
    pub fn _MODE<T: BankController>(mmu: &mut MMU<T>, mode: GPUMode) {
        let stat = mmu.read(ioregs::STAT) & 0b11111100;
        mmu.write(ioregs::STAT, stat | match mode { 
                GPUMode::HBLANK => 0, GPUMode::VBLANK => 1, GPUMode::OAM_SEARCH => 2, GPUMode::LCD_TRANSFER => 3, 
        });
    }

    // BG PALETTE GETTRS
    pub fn BG_COLOR_0_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::BGP) >> 0) & 0x03 }
    pub fn BG_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::BGP) >> 2) & 0x03 }
    pub fn BG_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::BGP) >> 4) & 0x03 }
    pub fn BG_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::BGP) >> 6) & 0x03 }

    // BG PALETTE SETTERS
    pub fn _BG_COLOR_0_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let bgp = mmu.read(ioregs::BGP) | ((color & 0x03) << 0); mmu.write(ioregs::BGP, bgp);
    }
    pub fn _BG_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let bgp = mmu.read(ioregs::BGP) | ((color & 0x03) << 2); mmu.write(ioregs::BGP, bgp);
    }
    pub fn _BG_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8){
        let bgp = mmu.read(ioregs::BGP) | ((color & 0x03) << 4); mmu.write(ioregs::BGP, bgp);
    }
    pub fn _BG_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let bgp = mmu.read(ioregs::BGP) | ((color & 0x03) << 6); mmu.write(ioregs::BGP, bgp);
    }

    // OBP0 PALETTE GETTERS
    pub fn OBP0_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::OBP_0) >> 2) & 0x03 }
    pub fn OBP0_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::OBP_0) >> 4) & 0x03 }
    pub fn OBP0_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::OBP_0) >> 6) & 0x03 }

    // OBP0 PALETTE SETTERS
    pub fn _OBP0_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_0) | ((color & 0x03) << 2); mmu.write(ioregs::OBP_0, obp);
    }
    pub fn _OBP0_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8){
        let obp = mmu.read(ioregs::OBP_0) | ((color & 0x03) << 4); mmu.write(ioregs::OBP_0, obp);
    }
    pub fn _OBP0_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_0) | ((color & 0x03) << 6); mmu.write(ioregs::OBP_0, obp);
    }

    // OBP1 PALETTE GETTERS
    pub fn OBP1_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::OBP_1) >> 2) & 0x03 }
    pub fn OBP1_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::OBP_1) >> 4) & 0x03 }
    pub fn OBP1_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 { (mmu.read(ioregs::OBP_1) >> 6) & 0x03 }

    // OBP1 PALETTE SETTERS
    pub fn _OBP1_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_1) | ((color & 0x03) << 2); mmu.write(ioregs::OBP_1, obp);
    }
    pub fn _OBP1_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8){
        let obp = mmu.read(ioregs::OBP_1) | ((color & 0x03) << 4); mmu.write(ioregs::OBP_1, obp);
    }
    pub fn _OBP1_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_1) | ((color & 0x03) << 6); mmu.write(ioregs::OBP_1, obp);
    }
    
    // Color translations based on current flags.
    pub fn bg_color<T: BankController>(mmu: &mut MMU<T>, color: u8) -> Color {
        get_color(match color {
            0 => GPU::BG_COLOR_0_SHADE(mmu),
            1 => GPU::BG_COLOR_1_SHADE(mmu),
            2 => GPU::BG_COLOR_2_SHADE(mmu), 
            3 => GPU::BG_COLOR_3_SHADE(mmu), 
            _ => 0xFF 
        })
    }

    pub fn obp0_color<T: BankController>(mmu: &mut MMU<T>, color: u8) -> Color  {
        if color == 0 {
            return TRANSPARENT 
        }
        get_color(match color {
            1 => GPU::OBP0_COLOR_1_SHADE(mmu), 
            2 => GPU::OBP0_COLOR_2_SHADE(mmu), 
            3 => GPU::OBP0_COLOR_3_SHADE(mmu),
            _ => 0x80 
        })
    }

    pub fn obp1_color<T: BankController>(mmu: &mut MMU<T>, color: u8) -> Color {
         if color == 0 { 
            return TRANSPARENT 
        }
        get_color(match color { 
            1 => GPU::OBP1_COLOR_1_SHADE(mmu), 
            2 => GPU::OBP1_COLOR_2_SHADE(mmu), 
            3 => GPU::OBP1_COLOR_3_SHADE(mmu), 
            _ => 0x40 
        })
    }
}