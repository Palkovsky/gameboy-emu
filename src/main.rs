pub mod mem;
pub use mem::*;
pub mod utils;
pub use utils::*;
pub mod dev;
pub use dev::*;
pub mod state;
pub use state::*;

use std::io::prelude::*;
use std::time::{Instant, Duration};
use std::{env, fs, io, thread};

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::rect::Rect;
use sdl2::audio::{AudioQueue, AudioSpecDesired};

const WINDOW_NAME: &str = "GAMEBOY EMU";
const SCALE: u32 = 5;
const FRAME_TIME: Duration = Duration::from_millis(1000/60);

fn main() {
    if env::args().len() != 2 {
        panic!("Usage: {} [rom]", env::args().nth(0).unwrap());
    }
    let path = env::args().nth(1).unwrap();
    let mut file = fs::File::open(path).unwrap();
    let mut rom = Vec::new();
    file.read_to_end(&mut rom).unwrap();

    let header = CartHeader::new(rom.iter()
        .take(0x150).skip(0x100)
        .map(|x| *x).collect());
    println!("{}", header);

    // Mapper type shouldn't be hardcoded here
    let mut runtime = Runtime::new(mbc::MBC3::new(rom));
    runtime.state.mmu.disable_bootrom();
    runtime.cpu.PC.set(0x100);
    
    let sdl_context = sdl2::init().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();
    let audio_spec = AudioSpecDesired { freq: Some(apu::PLAYBACK_FREQUENCY as i32), channels: Some(1), samples: Some(apu::BUFF_SIZE as u16) };
    let q1 = audio_subsystem.open_queue::<i16, _>(None, &audio_spec).unwrap();
    let q2 = audio_subsystem.open_queue::<i16, _>(None, &audio_spec).unwrap();

    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window(WINDOW_NAME, SCALE * SCREEN_WIDTH as u32, SCALE * SCREEN_HEIGHT as u32)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())
        .unwrap();
    let mut events = sdl_context.event_pump().unwrap();
    let mut canvas = window
        .into_canvas()
        .software()
        .build()
        .map_err(|e| e.to_string()).unwrap();

    'emulating: loop {
        let frame_start = Instant::now();
        let now = Instant::now();
        
        // CPU, GPU and other devices emulated here.
        while runtime.cpu_cycles() < CPU_CYCLES_PER_FRAME {
            runtime.step(); 
            let apu = &mut runtime.state.apu;
            play_samples(&q1, apu.chan1_samples());
            play_samples(&q2, apu.chan2_samples());
        }
        runtime.reset_cycles();
        // Print how long internal updates took        
        println!("Internal: {}ms", now.elapsed().as_millis());

        // Measure how long SDL part takes
        let now = Instant::now();
        // Handle events stream
        for event in events.poll_iter() {
            if let Event::Quit {..}  |  Event::KeyDown { keycode: Some(Keycode::Escape), .. } = event {
                 break 'emulating; 
            }
        }
        // Poll keyboard for button updates
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

        // Render current state of GPU framebuffer
        let gpu = &mut runtime.state.gpu;
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();
        for (i, (r, g, b)) in gpu.framebuff.iter().enumerate() {
            let y = i/SCREEN_WIDTH;
            let x = i%SCREEN_WIDTH;
            let rect = Rect::new(SCALE as i32 * x as i32, SCALE as i32 * y as i32, SCALE, SCALE);

            canvas.set_draw_color(Color::RGB(*r, *g, *b));
            canvas.fill_rect(rect).unwrap();
        }
        canvas.present();
        println!("Render : {}ms", now.elapsed().as_millis());

        // If some time left, sleep to get refresh rate of 60Hz
        if let Some(sleep_time) = FRAME_TIME.checked_sub(frame_start.elapsed()) {
            println!("Sleeping extra: {}ms", sleep_time.as_millis());
            thread::sleep(sleep_time);
        }
        println!("---------------");
    }
}

fn play_samples(queue: &AudioQueue<i16>, samples: &mut Vec<i16>) {
    if samples.len() >= apu::BUFF_SIZE {
        let buff = &samples[(samples.len()-apu::BUFF_SIZE)..];
        queue.queue(&buff);
        queue.resume();
        samples.clear();
    }
}