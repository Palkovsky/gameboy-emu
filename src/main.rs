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
use sdl2::keyboard::{Keycode, Scancode};
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
        
        for _ in 0..12000 { runtime.step(); }

        for event in events.poll_iter() {
            if let Event::Quit {..}  |  Event::KeyDown { keycode: Some(Keycode::Escape), .. } = event {
                 break 'emulating; 
            }
        }

        let joypad = &mut runtime.state.joypad;
        let keyboard = events.keyboard_state();

        joypad.up(keyboard.is_scancode_pressed(Scancode::W) | keyboard.is_scancode_pressed(Scancode::Up));
        joypad.down(keyboard.is_scancode_pressed(Scancode::S) | keyboard.is_scancode_pressed(Scancode::Down));
        joypad.left(keyboard.is_scancode_pressed(Scancode::A) | keyboard.is_scancode_pressed(Scancode::Left));
        joypad.right(keyboard.is_scancode_pressed(Scancode::D) | keyboard.is_scancode_pressed(Scancode::Right));
        joypad.a(keyboard.is_scancode_pressed(Scancode::Z));
        joypad.b(keyboard.is_scancode_pressed(Scancode::X));
        joypad.select(keyboard.is_scancode_pressed(Scancode::Space));
        joypad.start(keyboard.is_scancode_pressed(Scancode::Return) | keyboard.is_scancode_pressed(Scancode::Return2));

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
