#![allow(non_snake_case, non_camel_case_types)]

use super::super::VRAM_ADDR;
use super::*;

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
pub const FRAME_CYCLES: u64 = SCANLINE_CYCLES * (VBLANK_HEIGHT + SCREEN_HEIGHT) as u64;

pub const SCANLINE_STEPS: u64 = 3; // OAM -> LCD -> HBLANK -> (OAM -> LCD -> HBLANK ->)
pub const FRAME_STEPS: u64 = SCREEN_HEIGHT as u64 * SCANLINE_STEPS + 1;

pub const TILE_MAP_1: u16 = 0x9800;
pub const TILE_MAP_2: u16 = 0x9C00;
pub const TILE_BLOCK_1: u16 = 0x8000;
pub const TILE_BLOCK_2: u16 = 0x9000;
pub const TILE_SIZE: u16 = 16;
pub const SPRITE_COUNT: usize = 40;
pub const SCANLINE_SPRITE_COUNT: usize = 10;

pub type Color = (u8, u8, u8);
pub const WHITE: Color = (255, 255, 255);
pub const LIGHT_GRAY: Color = (184, 184, 184);
pub const DARK_GRAY: Color = (115, 115, 155);
pub const BLACK: Color = (0, 0, 0);
pub const TRANSPARENT: Color = (0, 255, 0);

fn get_color(num: u8) -> Color {
    match num {
        0 => WHITE,
        1 => LIGHT_GRAY,
        2 => DARK_GRAY,
        3 => BLACK,
        _ => panic!("Invalid color {}. Only 0, 1, 2, 3 are valid colors.", num),
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Sprite {
    y: u8,
    x: u8,
    tile_idx: u8,
    priority: bool,
    y_flip: bool,
    x_flip: bool,
    palette: bool,
}

fn read_oam(mmu: &mut MMU<impl BankController>, sprites: &mut [Sprite; SPRITE_COUNT]) {
    let oam = &mmu.oam;
    let mut off = 0;
    for i in 0..SPRITE_COUNT {
        let sprite: &mut Sprite = &mut sprites[i];
        sprite.y = oam[off];
        sprite.x = oam[off + 1];
        sprite.tile_idx = oam[off + 2];
        let flg = oam[off + 3];
        sprite.priority = flg & 0x80 != 0;
        sprite.y_flip = flg & 0x40 != 0;
        sprite.x_flip = flg & 0x20 != 0;
        sprite.palette = flg & 0x10 != 0;
        off += 4;
    }
    sprites.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
}

#[derive(Debug, PartialEq)]
pub enum GPUMode {
    HBLANK,
    VBLANK,
    OAM_SEARCH,
    LCD_TRANSFER,
}

impl Default for GPUMode {
    fn default() -> Self {
        GPUMode::OAM_SEARCH
    }
}

pub struct GPU {
    ly: u8,
    lx: u8,
    /* Keeps track of number of window lines rendered */
    wy: u8,
    /* Indicates wheater the window was drawn on current scanline */
    win_rendered: bool,
    pub sprites: [Sprite; SPRITE_COUNT],
    sprites_line: [usize; SCANLINE_SPRITE_COUNT],
    pub framebuff: Vec<Color>,
}

impl<T: BankController> Clocked<T> for GPU {
    fn next_time(&self, mmu: &mut MMU<T>) -> u64 {
        match GPU::MODE(mmu) {
            GPUMode::OAM_SEARCH => OAM_SEARCH_CYCLES,
            GPUMode::LCD_TRANSFER => 1,
            GPUMode::HBLANK => HBLANK_CYCLES,
            GPUMode::VBLANK => SCANLINE_CYCLES,
        }
    }

    fn step(&mut self, mmu: &mut MMU<T>) {
        self.update_ly(mmu);
        match GPU::MODE(mmu) {
            GPUMode::OAM_SEARCH => {
                read_oam(mmu, &mut self.sprites);
                self.oam_scanline(mmu);
                GPU::_MODE(mmu, GPUMode::LCD_TRANSFER);
            }
            GPUMode::LCD_TRANSFER => {
                for _ in 0..4 {
                    if GPU::LCD_DISPLAY_ENABLE(mmu) {
                        self.draw_dot(mmu);
                    }
                    self.lx += 1;
                }
                if self.lx == SCREEN_WIDTH as u8 {
                    GPU::_MODE(mmu, GPUMode::HBLANK);
                    GPU::hblank_stat_int(mmu);
                }
            }
            GPUMode::HBLANK => {
                self.lx = 0;
                self.ly += 1;
                if self.win_rendered {
                    self.win_rendered = false;
                    self.wy += 1;
                }
                self.update_ly(mmu);
                GPU::lyc_stat_int(mmu);
                if self.ly == SCREEN_HEIGHT as u8 {
                    GPU::_MODE(mmu, GPUMode::VBLANK);
                    GPU::vblank_int(mmu);
                    GPU::vblank_stat_int(mmu);
                } else {
                    GPU::_MODE(mmu, GPUMode::OAM_SEARCH);
                    GPU::oam_stat_int(mmu);
                }
            }
            GPUMode::VBLANK => {
                self.lx = 0;
                self.ly = if self.ly as usize == SCREEN_HEIGHT + VBLANK_HEIGHT {
                    0
                } else {
                    self.ly + 1
                };
                self.update_ly(mmu);
                GPU::lyc_stat_int(mmu);
                if self.ly == 0 {
                    self.wy = 0;
                    GPU::_MODE(mmu, GPUMode::OAM_SEARCH);
                    GPU::oam_stat_int(mmu);
                }
            }
        };
    }
}

impl GPU {
    pub fn new(mmu: &mut MMU<impl BankController>) -> Self {
        let mut res = Self {
            lx: 0,
            ly: 0,
            wy: 0,
            win_rendered: false,
            sprites: [Default::default(); SPRITE_COUNT],
            sprites_line: [0xFF; SCANLINE_SPRITE_COUNT],
            framebuff: vec![WHITE; SCREEN_WIDTH * SCREEN_HEIGHT],
        };
        GPU::_LCD_DISPLAY_ENABLE(mmu, true);
        GPU::_MODE(mmu, GPUMode::OAM_SEARCH);
        res.update_ly(mmu);
        res
    }

    // Fillup sprites_line with pointers to sprites on current line
    fn oam_scanline(&mut self, mmu: &mut MMU<impl BankController>) {
        let y = self.ly + 16;
        let h = if GPU::SPRITE_SIZE(mmu) { 16 } else { 8 };
        let mut j = 0;

        for i in 0..SPRITE_COUNT {
            if j == SCANLINE_SPRITE_COUNT {
                return;
            }
            let sprite = self.sprites[i];
            if y >= sprite.y && y < sprite.y + h {
                self.sprites_line[j] = i;
                j += 1;
            }
        }

        for i in j..SCANLINE_SPRITE_COUNT {
            self.sprites_line[i] = 0xFF;
        }
    }

    fn draw_window(&mut self, mmu: &mut MMU<impl BankController>) {
        let lx = self.lx as usize + 7;
        let ly = self.ly as usize;
        let wx = GPU::WX(mmu) as usize;
        let wy = GPU::WY(mmu) as usize;

        let in_window = ly >= wy && lx >= wx;
        if !in_window {
            return;
        }
        self.win_rendered = true;

        let tile_addressing = GPU::TILE_ADDRESSING(mmu);
        let tile_map = (if GPU::WINDOW_TILE_MAP(mmu) {
            TILE_MAP_2
        } else {
            TILE_MAP_1
        } - VRAM_ADDR) as usize;

        let (x, y) = (lx - wx, self.wy as usize);

        // Find tile coordinates
        let x_tile = x / 8;
        let y_tile = y / 8;
        let off = (32*y_tile + x_tile) % 1024;
        let tile_no = mmu.vram[tile_map + off];

        // By using tile number, fetch tile data from VRAM
        let tile_addr = match (tile_addressing, tile_no) {
            // 8000-8FFF unsigned addressing
            (true, tile) => TILE_BLOCK_1 + TILE_SIZE * (tile as u16),
            // 8800 signed addressing
            (false, tile) if (tile as i8) >= 0 => TILE_BLOCK_2 + TILE_SIZE * (tile as u16),
            (false, tile) if (tile as i8) < 0 => {
                TILE_BLOCK_2 - TILE_SIZE * ((-((tile as i8) as i16)) as u16)
            }
            // Won't happen
            (a, b) => panic!("Invalid tile addressing pattern: ({}, {})", a, b),
        } - VRAM_ADDR as u16;

        let start = tile_addr as usize;
        let end = start + TILE_SIZE as usize;
        let tile = &mmu.vram[start..end];

        // Which row we want to render?
        let tile_row = (y - y_tile * 8) as usize;
        let (b1, b2) = (tile[2 * tile_row], tile[2 * tile_row + 1]);

        // Which col we want to render?
        let tile_col = (x - x_tile * 8) as u16;
        let color = GPU::bytes_to_color_num(b1, b2, tile_col);
        let pixel_idx = ly*SCREEN_WIDTH + lx - 7;

        if pixel_idx < self.framebuff.len() {
            self.framebuff[pixel_idx] = GPU::bg_color(mmu, color);
        }
    }

    fn draw_background(&mut self, mmu: &mut MMU<impl BankController>) {
        let lx = self.lx as usize;
        let ly = self.ly as usize;
        let scx = GPU::SCX(mmu) as usize;
        let scy = GPU::SCY(mmu) as usize;

        let tile_addressing = GPU::TILE_ADDRESSING(mmu);
        let tile_map = (if GPU::BG_TILE_MAP(mmu) {
            TILE_MAP_2
        } else {
            TILE_MAP_1
        } - VRAM_ADDR) as usize;

        // Coordinates of tile to fetch.
        let (x, y) = ((scx + lx) % 256, (scy + ly) % 256);

        // Find tile coordinates
        let x_tile = x / 8;
        let y_tile = y / 8;
        let off = (32*y_tile + x_tile) % 1024;
        let tile_no = mmu.vram[tile_map + off];

        // By using tile number, fetch tile data from VRAM
        let tile_addr = match (tile_addressing, tile_no) {
            // 8000-8FFF unsigned addressing
            (true, tile) => TILE_BLOCK_1 + TILE_SIZE * (tile as u16),
            // 8800 signed addressing
            (false, tile) if (tile as i8) >= 0 => TILE_BLOCK_2 + TILE_SIZE * (tile as u16),
            (false, tile) if (tile as i8) < 0 => {
                TILE_BLOCK_2 - TILE_SIZE * ((-((tile as i8) as i16)) as u16)
            }
            // Won't happen
            (a, b) => panic!("Invalid tile addressing pattern: ({}, {})", a, b),
        } - VRAM_ADDR as u16;

        let start = tile_addr as usize;
        let end = start + TILE_SIZE as usize;
        let tile = &mmu.vram[start..end];

        // Which row we want to render?
        let tile_row = (y - y_tile * 8) as usize;
        let (b1, b2) = (tile[2 * tile_row], tile[2 * tile_row + 1]);

        // Which col we want to render?
        let tile_col = (x - x_tile * 8) as u16;
        let color = GPU::bytes_to_color_num(b1, b2, tile_col);
        let pixel_idx = ly*SCREEN_WIDTH + lx;

        if pixel_idx < self.framebuff.len() {
            self.framebuff[pixel_idx] = GPU::bg_color(mmu, color);
        }
    }

    fn draw_sprite(&mut self, mmu: &mut MMU<impl BankController>){
        let sprite_h = if GPU::SPRITE_SIZE(mmu) { 16 } else { 8 };
        let sprite_w = 8;
        let lx = self.lx;
        let ly = self.ly;

        // Find sprite to draw
        let mut sprite_to_render = None;
        for i in self.sprites_line.iter() {
            let idx = *i;
            if idx == 0xFF {
                break;
            }

            let tmp = self.sprites[idx];
            if tmp.x > lx && tmp.x <= lx + sprite_w {
                sprite_to_render = Some(tmp);
                break;
            }
        }

        if let Some(sprite) = sprite_to_render {
            let vram = &mmu.vram[..];
            let mut sprite_row = (ly + 16) - sprite.y;
            if sprite.y_flip {
                sprite_row = sprite_h - sprite_row as u8;
            }

            let base_addr = if sprite_h == 16 {
                // 8x16 sprites
                let tile_idx = if sprite_row >= 8 {
                    sprite_row -= 8;
                    sprite.tile_idx | 0x01
                } else {
                    sprite.tile_idx & 0xFE
                };
                let tile_addr = TILE_BLOCK_1 + TILE_SIZE * (tile_idx as u16) - VRAM_ADDR;
                tile_addr as usize + 2 * sprite_row as usize
            } else {
                // 8x8 sprites
                let tile_addr = TILE_BLOCK_1 + TILE_SIZE * (sprite.tile_idx as u16) - VRAM_ADDR;
                tile_addr as usize + 2 * sprite_row as usize
            };

            // b1 and b2 are two bytes representing sprite tile
            let (b1, b2) = (vram[base_addr], vram[base_addr + 1]);

            // Locate specific pixel x coordinate
            let off = (lx + sprite_w) - sprite.x;
            let sprite_col = if sprite.x_flip { sprite_w - 1 - off } else { off };

            // Lookup color
            let color_idx = GPU::bytes_to_color_num(b1, b2, sprite_col as u16);
            let color = if sprite.palette {
                GPU::obp1_color(mmu, color_idx)
            } else {
               GPU::obp0_color(mmu, color_idx)
            };

            let pixel_idx = ly as usize * SCREEN_WIDTH + lx as usize;

            // Handle sprite priority
            let bg_color_0_id = GPU::BG_COLOR_0_SHADE(mmu);
            let bg_color_0 = GPU::bg_color(mmu, bg_color_0_id);
            if sprite.priority && self.framebuff[pixel_idx] != bg_color_0 {
                return;
            }

            // Put it in the framebuff
            if pixel_idx < self.framebuff.len() && color != TRANSPARENT {
                self.framebuff[pixel_idx] = color;
            }
        }
    }

    fn draw_dot(&mut self, mmu: &mut MMU<impl BankController>){
        if GPU::DISPLAY_PRIORITY(mmu) {
            self.draw_background(mmu);
            if GPU::WINDOW_ENABLED(mmu) {
                self.draw_window(mmu);
            }
        }
        if GPU::SPRITE_ENABLED(mmu) {
            self.draw_sprite(mmu);
        }
    }

    // update_ly() performs LY=LYC check, updates COINCIDENCE FLAG and (optionally) triggers STAT interrupt.
    pub fn update_ly(&mut self, mmu: &mut MMU<impl BankController>) {
        let lyc = GPU::LYC(mmu);
        GPU::_LY(mmu, self.ly);
        GPU::_COINCIDENCE_FLAG(mmu, self.ly == lyc);
    }

    fn vblank_stat_int(mmu: &mut MMU<impl BankController>) {
        if GPU::MODE_1_VBLANK_INTERRUPT_ENABLE(mmu) {
            GPU::stat_int(mmu);
        }
    }

    fn hblank_stat_int(mmu: &mut MMU<impl BankController>) {
        if GPU::MODE_0_HBLANK_INTERRUPT_ENABLE(mmu) {
            GPU::stat_int(mmu);
        }
    }

    fn oam_stat_int(mmu: &mut MMU<impl BankController>) {
        if GPU::MODE_2_OAM_INTERRUPT_ENABLE(mmu) {
            GPU::stat_int(mmu);
        }
    }

    fn lyc_stat_int(mmu: &mut MMU<impl BankController>) {
        if GPU::COINCIDENCE_INTERRUPT_ENABLE(mmu) && GPU::COINCIDENCE_FLAG(mmu){
            GPU::stat_int(mmu);
        }
    }

    // Triggers VBLANK interrupt
    fn vblank_int(mmu: &mut MMU<impl BankController>) {
        if Self::LCD_DISPLAY_ENABLE(mmu) {
            mmu.set_bit(ioregs::IF, 0, true);
        }
    }
    // Triggers STAT interrupt
    fn stat_int(mmu: &mut MMU<impl BankController>) {
        if Self::LCD_DISPLAY_ENABLE(mmu) {
            mmu.set_bit(ioregs::IF, 1, true);
        }
    }

    pub fn LY<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        mmu.read(ioregs::LY)
    }
    pub fn LYC<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        mmu.read(ioregs::LYC)
    }
    pub fn WX<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        mmu.read(ioregs::WX)
    }
    pub fn WY<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        mmu.read(ioregs::WY)
    }
    pub fn SCX<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        mmu.read(ioregs::SCX)
    }
    pub fn SCY<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        mmu.read(ioregs::SCY)
    }

    pub fn _LY<T: BankController>(mmu: &mut MMU<T>, val: u8) {
        mmu.write(ioregs::LY, val);
    }

    // LCDC GETTERS
    /* (0=Off, 1=On) */
    pub fn LCD_DISPLAY_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::LCDC, 7)
    }
    /* (0=9800-9BFF, 1=9C00-9FFF) */
    pub fn WINDOW_TILE_MAP<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::LCDC, 6)
    }
    /* (0=Off, 1=On) */
    pub fn WINDOW_ENABLED<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::LCDC, 5)
    }
    /* (0=8800-97FF, 1=8000-8FFF) For sprites it's always 8000-8FFF */
    pub fn TILE_ADDRESSING<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::LCDC, 4)
    }
    /* (0=9800-9BFF, 1=9C00-9FFF) */
    pub fn BG_TILE_MAP<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::LCDC, 3)
    }
    /* (0=8x8, 1=8x16) */
    pub fn SPRITE_SIZE<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::LCDC, 2)
    }
    /* 0=Off, 1=On) */
    pub fn SPRITE_ENABLED<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::LCDC, 1)
    }
    /* (0=Off, 1=On) */
    pub fn DISPLAY_PRIORITY<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::LCDC, 0)
    }

    // LCDC SETTERS
    pub fn _LCD_DISPLAY_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::LCDC, 7, flg)
    }
    pub fn _WINDOW_TILE_MAP<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::LCDC, 6, flg)
    }
    pub fn _WINDOW_ENABLED<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::LCDC, 5, flg)
    }
    pub fn _TILE_ADDRESSING<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::LCDC, 4, flg)
    }
    pub fn _BG_TILE_MAP<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::LCDC, 3, flg)
    }
    pub fn _SPRITE_SIZE<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::LCDC, 2, flg)
    }
    pub fn _SPRITE_ENABLED<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::LCDC, 1, flg)
    }
    pub fn _DISPLAY_PRIORITY<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::LCDC, 0, flg)
    }

    // STAT GETTERS
    pub fn COINCIDENCE_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::STAT, 6)
    }
    pub fn MODE_2_OAM_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::STAT, 5)
    }
    pub fn MODE_1_VBLANK_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::STAT, 4)
    }
    pub fn MODE_0_HBLANK_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::STAT, 3)
    }
    pub fn COINCIDENCE_FLAG<T: BankController>(mmu: &mut MMU<T>) -> bool {
        mmu.read_bit(ioregs::STAT, 2)
    }
    pub fn MODE<T: BankController>(mmu: &mut MMU<T>) -> GPUMode {
        match mmu.read(ioregs::STAT) & 0x3 {
            0 => GPUMode::HBLANK,
            1 => GPUMode::VBLANK,
            2 => GPUMode::OAM_SEARCH,
            _ => GPUMode::LCD_TRANSFER,
        }
    }

    // STAT SETTERS
    pub fn _COINCIDENCE_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::STAT, 6, flg)
    }
    pub fn _MODE_2_OAM_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::STAT, 5, flg)
    }
    pub fn _MODE_1_VBLANK_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::STAT, 4, flg)
    }
    pub fn _MODE_0_HBLANK_INTERRUPT_ENABLE<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::STAT, 3, flg)
    }
    pub fn _COINCIDENCE_FLAG<T: BankController>(mmu: &mut MMU<T>, flg: bool) {
        mmu.set_bit(ioregs::STAT, 2, flg)
    }
    pub fn _MODE<T: BankController>(mmu: &mut MMU<T>, mode: GPUMode) {
        let stat = mmu.read(ioregs::STAT) & 0b11111100;
        mmu.write(
            ioregs::STAT,
            stat | match mode {
                GPUMode::HBLANK => 0,
                GPUMode::VBLANK => 1,
                GPUMode::OAM_SEARCH => 2,
                GPUMode::LCD_TRANSFER => 3,
            },
        );
    }

    // BG PALETTE GETTRS
    pub fn BG_COLOR_0_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::BGP) >> 0) & 0x03
    }
    pub fn BG_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::BGP) >> 2) & 0x03
    }
    pub fn BG_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::BGP) >> 4) & 0x03
    }
    pub fn BG_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::BGP) >> 6) & 0x03
    }

    // BG PALETTE SETTERS
    pub fn _BG_COLOR_0_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let bgp = mmu.read(ioregs::BGP) | ((color & 0x03) << 0);
        mmu.write(ioregs::BGP, bgp);
    }
    pub fn _BG_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let bgp = mmu.read(ioregs::BGP) | ((color & 0x03) << 2);
        mmu.write(ioregs::BGP, bgp);
    }
    pub fn _BG_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let bgp = mmu.read(ioregs::BGP) | ((color & 0x03) << 4);
        mmu.write(ioregs::BGP, bgp);
    }
    pub fn _BG_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let bgp = mmu.read(ioregs::BGP) | ((color & 0x03) << 6);
        mmu.write(ioregs::BGP, bgp);
    }

    // OBP0 PALETTE GETTERS
    pub fn OBP0_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::OBP_0) >> 2) & 0x03
    }
    pub fn OBP0_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::OBP_0) >> 4) & 0x03
    }
    pub fn OBP0_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::OBP_0) >> 6) & 0x03
    }

    // OBP0 PALETTE SETTERS
    pub fn _OBP0_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_0) | ((color & 0x03) << 2);
        mmu.write(ioregs::OBP_0, obp);
    }
    pub fn _OBP0_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_0) | ((color & 0x03) << 4);
        mmu.write(ioregs::OBP_0, obp);
    }
    pub fn _OBP0_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_0) | ((color & 0x03) << 6);
        mmu.write(ioregs::OBP_0, obp);
    }

    // OBP1 PALETTE GETTERS
    pub fn OBP1_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::OBP_1) >> 2) & 0x03
    }
    pub fn OBP1_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::OBP_1) >> 4) & 0x03
    }
    pub fn OBP1_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>) -> u8 {
        (mmu.read(ioregs::OBP_1) >> 6) & 0x03
    }

    // OBP1 PALETTE SETTERS
    pub fn _OBP1_COLOR_1_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_1) | ((color & 0x03) << 2);
        mmu.write(ioregs::OBP_1, obp);
    }
    pub fn _OBP1_COLOR_2_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_1) | ((color & 0x03) << 4);
        mmu.write(ioregs::OBP_1, obp);
    }
    pub fn _OBP1_COLOR_3_SHADE<T: BankController>(mmu: &mut MMU<T>, color: u8) {
        let obp = mmu.read(ioregs::OBP_1) | ((color & 0x03) << 6);
        mmu.write(ioregs::OBP_1, obp);
    }

    // Color translations based on current flags.
    pub fn bg_color<T: BankController>(mmu: &mut MMU<T>, color: u8) -> Color {
        get_color(match color {
            0 => GPU::BG_COLOR_0_SHADE(mmu),
            1 => GPU::BG_COLOR_1_SHADE(mmu),
            2 => GPU::BG_COLOR_2_SHADE(mmu),
            3 => GPU::BG_COLOR_3_SHADE(mmu),
            _ => 0xFF,
        })
    }

    pub fn obp0_color<T: BankController>(mmu: &mut MMU<T>, color: u8) -> Color {
        if color == 0 {
            return TRANSPARENT;
        }
        get_color(match color {
            1 => GPU::OBP0_COLOR_1_SHADE(mmu),
            2 => GPU::OBP0_COLOR_2_SHADE(mmu),
            3 => GPU::OBP0_COLOR_3_SHADE(mmu),
            _ => 0x80,
        })
    }

    pub fn obp1_color<T: BankController>(mmu: &mut MMU<T>, color: u8) -> Color {
        if color == 0 {
            return TRANSPARENT;
        }
        get_color(match color {
            1 => GPU::OBP1_COLOR_1_SHADE(mmu),
            2 => GPU::OBP1_COLOR_2_SHADE(mmu),
            3 => GPU::OBP1_COLOR_3_SHADE(mmu),
            _ => 0x40,
        })
    }

    fn bytes_to_color_num(b1: u8, b2: u8, off: u16) -> u8 {
        let mask = 0x80 >> off;
        match (b2 & mask != 0, b1 & mask != 0) {
            (true, true) => 3,
            (true, false) => 2,
            (false, true) => 1,
            (false, false) => 0,
        }
    }
}
