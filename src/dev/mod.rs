pub mod cpu;
pub use cpu::*;

pub mod gpu;
pub use gpu::*;

pub mod apu;
pub use apu::*;

pub mod timer;
pub use timer::*;

pub mod dma;
pub use dma::*;

pub mod joypad;
pub use joypad::*;

use super::{BankController, MMU, State};
use super::mem::ioregs;

pub trait Clocked<T: BankController> {
    /*
     * next_time()
     * Returns number of clocks of next step() operation.
     */
    fn next_time(&self, mmu: &mut MMU<T>) -> u64;

    /*
     * Performs update taking expected number of clocks.
     */
    fn step(&mut self, mmu: &mut MMU<T>);
} 