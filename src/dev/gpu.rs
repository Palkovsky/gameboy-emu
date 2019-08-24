#![allow(non_snake_case, non_camel_case_types)]

use super::*;
use super::super::{VRAM_ADDR};

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;
pub const VBLANK_HEIGHT: usize = 10;

/*
 * MODE 0 - HBLANK
 * MODE 1 - VBLANK
 * MODE 2 - OAM SEARCH
 * MODE 3 - LCD TRANSFER
 */

const OAM_SEARCH_CYCLES: u64 = 20;
const LCD_TRANSFER_CYCLES: u64 = 43;
const HBLANK_CYCLES: u64 = 51;
const SCANLINE_CYCLES: u64 = OAM_SEARCH_CYCLES + LCD_TRANSFER_CYCLES + HBLANK_CYCLES;
const VBLANK_CYCLES: u64 = SCANLINE_CYCLES * VBLANK_HEIGHT as u64;
const FRAME_CYCLES: u64 = SCANLINE_CYCLES * (SCREEN_HEIGHT + VBLANK_HEIGHT) as u64;

pub const SCANLINE_STEPS: u64 = 3; // OAM -> LCD -> HBLANK -> (OAM -> LCD -> HBLANK ->)
pub const FRAME_STEPS: u64 = SCREEN_HEIGHT as u64*SCANLINE_STEPS + 1;

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
pub enum GPUMode {
    HBLANK, VBLANK, OAM_SEARCH, LCD_TRANSFER,
}
impl Default for GPUMode {
    fn default() -> Self { GPUMode::OAM_SEARCH }
}

pub struct GPU {
    ly: u8,
    pub framebuff: Vec<Color>,
}

impl <T: BankController>Clocked<T> for GPU {
    fn next_time(&self, mmu: &mut MMU<T>) -> u64 {
        if !GPU::LCD_DISPLAY_ENABLE(mmu) { return 0 }
        match GPU::MODE(mmu) {
            GPUMode::OAM_SEARCH => OAM_SEARCH_CYCLES,
            GPUMode::LCD_TRANSFER => LCD_TRANSFER_CYCLES,
            GPUMode::HBLANK => HBLANK_CYCLES,
            GPUMode::VBLANK => VBLANK_CYCLES,
        } 
    }

    fn step(&mut self, mmu: &mut MMU<T>) {
        self.update(mmu);
        match GPU::MODE(mmu) {
            GPUMode::OAM_SEARCH => {
                if GPU::COINCIDENCE_INTERRUPT_ENABLE(mmu) && GPU::COINCIDENCE_FLAG(mmu) {
                    GPU::stat_int(mmu);
                }
                GPU::_MODE(mmu, GPUMode::LCD_TRANSFER);
            },
            GPUMode::LCD_TRANSFER => {
                self.scanline(mmu);
                GPU::_MODE(mmu, GPUMode::HBLANK);
            },
            GPUMode::HBLANK => {
                self.ly += 1;
                self.update(mmu);

                if self.ly == SCREEN_HEIGHT as u8 {
                    GPU::_MODE(mmu, GPUMode::VBLANK);
                    GPU::vblank_int(mmu);
                    if GPU::MODE_1_VBLANK_INTERRUPT_ENABLE(mmu) { GPU::stat_int(mmu); }
                } else {
                    GPU::_MODE(mmu, GPUMode::OAM_SEARCH);
                    if GPU::MODE_2_OAM_INTERRUPT_ENABLE(mmu) { GPU::stat_int(mmu); }
                }
            },
            GPUMode::VBLANK => {
                GPU::_MODE(mmu, GPUMode::OAM_SEARCH);
                self.ly = 0;
                self.update(mmu);
                if GPU::MODE_2_OAM_INTERRUPT_ENABLE(mmu) { GPU::stat_int(mmu); }
            },
        };
    }
}

impl GPU {
    pub fn new<T: BankController>(mmu: &mut MMU<T>) -> Self {
        let mut res = Self {
            ly: 0,
            framebuff: vec![WHITE; SCREEN_WIDTH*SCREEN_HEIGHT],
        };
      
        GPU::_LCD_DISPLAY_ENABLE(mmu, true);
        GPU::_MODE(mmu, GPUMode::OAM_SEARCH);
        res.update(mmu);
        res
    }

    // Draws LY scanline.
    fn scanline<T: BankController>(&mut self, mmu: &mut MMU<T>) { 
        let mut lx = 0usize;
        let ly = self.ly as usize;
        
        let scx = GPU::SCX(mmu) as usize;
        let scy = GPU::SCY(mmu) as usize;

        let wx = GPU::WX(mmu) as usize;
        let wy = GPU::WY(mmu) as usize;
        let win_enabled = GPU::WINDOW_ENABLED(mmu) &&
                          ly >= wy && 
                          wx >= 7 &&
                          wx <= SCREEN_WIDTH + 7 &&
                          wy <= SCREEN_HEIGHT;
        let in_window = |lx: usize| win_enabled && lx >= wx - 7;

        let tile_addressing = GPU::TILE_ADDRESSING(mmu);
        let window_tile_map = (if GPU::WINDOW_TILE_MAP(mmu) 
            { TILE_MAP_2 } else { TILE_MAP_1 } - VRAM_ADDR) as usize;
        let bg_tile_map = (if GPU::BG_TILE_MAP(mmu) 
            { TILE_MAP_2 } else { TILE_MAP_1 } - VRAM_ADDR) as usize;

        while lx < SCREEN_WIDTH {
            let window = in_window(lx);

            // Coordinates of tile to fetch.
            let (x, y, tile_map) = match window {
                true => (lx + 7, ly, window_tile_map), // Not sure if it should be 'lx+7' or jsut 'lx'
                false => ((scx + lx) % 256, (scy + ly) % 256, bg_tile_map),                
            };

            let x_tile = x/8;
            let y_tile = y/8;
            let off = (32*y_tile + x_tile) % 1024;
            let tile_no = mmu.vram[tile_map + off] as u16;

            // By using tile number, fetch tile data from VRAM
            let tile_addr = match (tile_addressing, tile_no) {
                // 8000-8FFF unsigned addressing
                (true, tile) => TILE_BLOCK_1 + TILE_SIZE*tile,
                // 8800 signed addressing
                (false, tile) if tile < 0x80 => TILE_BLOCK_2 + TILE_SIZE*tile,
                (false, tile) if tile >= 0x80 => TILE_BLOCK_2 - TILE_SIZE*(tile - 0x80),
                // Won't happen
                (a, b) => { panic!("Invalid tile addressing pattern: ({}, {})", a, b) }
            } - VRAM_ADDR as u16;

            // Not using read/write MMU methods here, cuz 
            let start = tile_addr as usize;
            let end = start + TILE_SIZE as usize;
            let tile = &mmu.vram[start..end];

            // Which row we want to render?
            let tile_row = (y - y_tile*8) as usize;
            let (b1, b2) = (tile[2*tile_row], tile[2*tile_row+1]);

            // Which col we want to render?
            let tile_col = (x - x_tile*8) as u16;

            //println!("LX {}, X {}", idx, lx, x);

            for off in tile_col..8 {    
                if lx >= SCREEN_WIDTH { break; }
                // When drawing background, but entered window area.
                if !window && in_window(lx) { break; }

                let mask = 0x80 >> off;
                let color = match (b2 & mask != 0, b1 & mask != 0) {
                    (true, true) => 3,
                    (true, false) => 2,
                    (false, true) => 1,
                    (false, false) => 0,
                };

                self.framebuff[ly*SCREEN_WIDTH + lx] = GPU::bg_color(mmu, color);
                lx += 1;
            }
        }
    }

    // update() performs LY=LYC check, updates COINCIDENCE FLAG and (optionally) triggers STAT interrupt.
    pub fn update<T: BankController>(&mut self, mmu: &mut MMU<T>) {
        let lyc = GPU::LYC(mmu);
        GPU::_LY(mmu, self.ly);
        GPU::_COINCIDENCE_FLAG(mmu, self.ly == lyc);
    }

    // Triggers VBLANK interrupt
    fn vblank_int<T: BankController>(mmu: &mut MMU<T>) { mmu.set_bit(ioregs::IF, 0, true); }
    // Triggers STAT interrupt
    fn stat_int<T: BankController>(mmu: &mut MMU<T>) { mmu.set_bit(ioregs::IF, 1, true); }

    pub fn LY<T: BankController>(mmu: &mut MMU<T>) -> u8 { mmu.read(ioregs::LY) }
    pub fn LYC<T: BankController>(mmu: &mut MMU<T>) -> u8 { mmu.read(ioregs::LYC) }
    pub fn WX<T: BankController>(mmu: &mut MMU<T>) -> u8 { mmu.read(ioregs::WX) }
    pub fn WY<T: BankController>(mmu: &mut MMU<T>) -> u8 { mmu.read(ioregs::WY) }
    pub fn SCX<T: BankController>(mmu: &mut MMU<T>) -> u8 { mmu.read(ioregs::SCX) }
    pub fn SCY<T: BankController>(mmu: &mut MMU<T>) -> u8 { mmu.read(ioregs::SCY) }

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