pub mod mem;
pub use mem::*;
pub mod utils;
pub use utils::*;
pub mod dev;
pub use dev::*;
pub mod state;
pub use state::*;

use std::io::prelude::*;
use std::time::{Instant};
use std::num::Wrapping;
use std::{env, fs, io};

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;

const WINDOW_NAME: &str = "GAMEBOY EMU";
const SCALE: u32 = 4;

fn main() {
    if env::args().len() != 2 {
        panic!("Usage: {} [rom]", env::args().nth(0).unwrap());
    }
    let path = env::args().nth(1).unwrap();
    let mut file = fs::File::open(path).unwrap();
    let mut rom = Vec::new();
    file.read_to_end(&mut rom).unwrap();

    //let header = CartHeader::new(rom.iter()
    //    .take(0x150).skip(0x100)
    //    .map(|x| *x).collect());
    //println!("{}", header);

    let mut runtime = Runtime::new(mbc::RomOnly::new(rom));
    runtime.state.mmu.disable_bootrom();
    runtime.cpu.PC.set(0x100);

    //let mut runtime = Runtime::new(mbc::RomOnly::new(rom));
    

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window(WINDOW_NAME, SCALE * SCREEN_WIDTH as u32, SCALE * SCREEN_HEIGHT as u32)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())
        .unwrap();
    let mut events = sdl_context.event_pump().unwrap();
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string()).unwrap();
    
    'emulating: loop {
        let now = Instant::now();
        
        for _ in 0..8000 { runtime.step(); }

        for event in events.poll_iter() {
            if let Event::Quit {..}  |  Event::KeyDown { keycode: Some(Keycode::Escape), .. } = event {
                 break 'emulating; 
            }

            let update = 4;
            let mmu = &mut runtime.state.mmu;
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

        let gpu = &mut runtime.state.gpu;
        let mmu = &mut runtime.state.mmu;

        //println!("{}ms/frame | Rs: {} | Ws: {}", now.elapsed().as_millis(), mmu.reads, mmu.writes);
        mmu.reads = 0;
        mmu.writes = 0;

        for (i, (r, g, b)) in gpu.framebuff.iter().enumerate() {
            let y = i/SCREEN_WIDTH;
            let x = i%SCREEN_WIDTH;
            let rect = Rect::new(SCALE as i32 * x as i32, SCALE as i32 * y as i32, SCALE, SCALE);

            canvas.set_draw_color(Color::RGB(*r, *g, *b));
            canvas.fill_rect(rect).unwrap();
        }
/*
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let rect = Rect::new(SCALE as i32 * x as i32*8, SCALE as i32 * y as i32*8, SCALE*8, SCALE*8);

                canvas.set_draw_color(Color::RGB(0, 255, 0));
                canvas.draw_rect(rect)?;
            }
        }
  */      
        canvas.present();
    } 
}
