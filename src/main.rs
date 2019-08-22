pub mod mem;
pub use mem::*;
pub mod utils;
pub use utils::*;
pub mod dev;
pub use dev::*;
pub mod state;
pub use state::*;

use std::time::{Instant};
use std::num::Wrapping;
use std::{env, fs, io};
use io::prelude::*;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;

const WINDOW_NAME: &str = "GAMEBOY EMU";
const SCALE: u32 = 4;
const A_CHAR: [u8; 16] = [
    0x7C, 0x7C, 0x00, 0xC6, 0xC6, 0x00, 0x00, 0xFE, 0xC6, 0xC6, 0x00, 0xC6, 0xC6, 0x00, 0x00, 0x00,
];
const BLACK_TILE: [u8; 16] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

fn main() -> Result<(), String> {
    /*
    if env::args().len() != 2 {
        panic!("Usage: {} [rom]", env::args().nth(0).unwrap());
    }
    let path = env::args().nth(1).unwrap();
    let mut file = fs::File::open(path).unwrap();
    let mut rom = Vec::new();
    file.read_to_end(&mut rom)?;

    let header = CartHeader::new(rom.iter()
        .take(0x150).skip(0x100)
        .map(|x| *x).collect());
    println!("{}", header);
    */

    let mut state = State::new(MBC1::new(vec![0; 1 << 21]));
    let mmu = &mut state.mmu;
    let gpu = &mut state.gpu;

    for (i, byte) in BLACK_TILE.iter().enumerate() { mmu.write(32 + 0x9000 + i as u16, *byte); }
    for (i, byte) in A_CHAR.iter().enumerate() { mmu.write(16 + 0x9000 + i as u16, *byte); }

    mmu.write(TILE_MAP_2, 2);
    mmu.write(TILE_MAP_2 + 20, 1);
    mmu.write(TILE_MAP_2 + 26, 1);
    mmu.write(TILE_MAP_2 + 30, 1);
    mmu.write(TILE_MAP_2 + 31, 1);
    mmu.write(TILE_MAP_2 + 32, 1);
    mmu.write(TILE_MAP_2 + 32*31, 1);
    mmu.write(TILE_MAP_2 + 32*32 - 1, 1);
    for i in 0..1024 { mmu.write(TILE_MAP_1 + i, 2); }
    
    mmu.write(SCX, 0);
    mmu.write(SCY, 0);
    mmu.write(WX, SCREEN_WIDTH as u8/2);
    mmu.write(WY, (SCREEN_HEIGHT as f64 * 0.25) as u8);
    mmu.write(BGP, 0b11100100);

    GPU::_LCD_DISPLAY_ENABLE(mmu, true);
    GPU::_WINDOW_ENABLED(mmu, true);
    GPU::_TILE_ADDRESSING(mmu, false);
    GPU::_BG_TILE_MAP(mmu, true);
    GPU::_WINDOW_TILE_MAP(mmu, false);

    for _ in 0..FRAME_CYCLES { gpu.step(mmu); }

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem.window(WINDOW_NAME, SCALE * SCREEN_WIDTH as u32, SCALE * SCREEN_HEIGHT as u32)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;
    let mut events = sdl_context.event_pump()?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    
    'emulating: loop {
        let now = Instant::now();
        for _ in 0..FRAME_STEPS { gpu.step(mmu); }

        println!("{}ms/frame | Rs: {} | Ws: {}", now.elapsed().as_millis(), mmu.reads, mmu.writes);
        mmu.reads = 0;
        mmu.writes = 0;

        for event in events.poll_iter() {
            if let Event::Quit {..}  |  Event::KeyDown { keycode: Some(Keycode::Escape), .. } = event {
                 break 'emulating; 
            }

            let update = 1;
            match event {
                // SCX/SCY controls
                Event::KeyDown { keycode: Some(Keycode::A), .. } => { 
                    let scx = Wrapping(mmu.read(SCX)) - Wrapping(update);
                    mmu.write(ioregs::SCX, scx.0);
                },
                Event::KeyDown { keycode: Some(Keycode::D), .. } => { 
                    let scx = Wrapping(mmu.read(SCX)) + Wrapping(update);
                    mmu.write(ioregs::SCX, scx.0);
                },
                Event::KeyDown { keycode: Some(Keycode::W), .. } => { 
                    let scy = Wrapping(mmu.read(SCY)) - Wrapping(update);
                    mmu.write(ioregs::SCY, scy.0);
                },
                Event::KeyDown { keycode: Some(Keycode::S), .. } => { 
                    let scy = Wrapping(mmu.read(SCY)) + Wrapping(update);
                    mmu.write(ioregs::SCY, scy.0);
                },

                // Window ON/OFF switch
                Event::KeyDown { keycode: Some(Keycode::F), .. } => {
                    let enabled = GPU::WINDOW_ENABLED(mmu);
                    GPU::_WINDOW_ENABLED(mmu, enabled ^ true);
                },

                // Window WX/WY controls
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => { 
                    let wx = Wrapping(mmu.read(WX)) - Wrapping(update);
                    mmu.write(ioregs::WX, wx.0);
                },
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => { 
                    let wx = Wrapping(mmu.read(WX)) + Wrapping(update);
                    mmu.write(ioregs::WX, wx.0);
                },
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => { 
                    let wy = Wrapping(mmu.read(WY)) - Wrapping(update);
                    mmu.write(ioregs::WY, wy.0);
                },
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => { 
                    let wy = Wrapping(mmu.read(WY)) + Wrapping(update);
                    mmu.write(ioregs::WY, wy.0);
                },
                _ => {}
            }
        }

        for (i, (r, g, b)) in gpu.framebuff.iter().enumerate() {
            let y = i/SCREEN_WIDTH;
            let x = i%SCREEN_WIDTH;
            let rect = Rect::new(SCALE as i32 * x as i32, SCALE as i32 * y as i32, SCALE, SCALE);

            canvas.set_draw_color(Color::RGB(*r, *g, *b));
            canvas.fill_rect(rect)?;
        }

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let rect = Rect::new(SCALE as i32 * x as i32*8, SCALE as i32 * y as i32*8, SCALE*8, SCALE*8);

                canvas.set_draw_color(Color::RGB(0, 255, 0));
                canvas.draw_rect(rect)?;
            }
        }
        
        canvas.present();
    } 

    Ok(())
}
