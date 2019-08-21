pub mod mmu;
pub mod mbc;
pub mod ioregs;

pub use mmu::*;
pub use mbc::*;
pub use ioregs::*;


pub type Addr = u16;
pub type Byte = u8;
pub type MutMem<'a> = &'a mut [Byte];

/*
 * Base addresses of different memory map segments.
 */ 
pub const ROM_BASE_ADDR: Addr = 0x0000;
pub const ROM_SWITCHABLE_ADDR: Addr = 0x4000;
pub const VRAM_ADDR: Addr = 0x8000;
pub const RAM_SWITCHABLE_ADDR: Addr = 0xA000;
pub const RAM_BASE_ADDR: Addr = 0xC000;
pub const RAM_ECHO_ADDR: Addr = 0xE000;
pub const OAM_ADDR: Addr = 0xFE00;
pub const STACK_ADDR: Addr = 0xFF80;
pub const IO_REGS_ADDR: Addr = 0xFF00;

pub const BOOSTRAP_SIZE: usize = 0x100;
pub const RAM_BANK_SIZE: usize = 0x2000;
pub const ROM_BANK_SIZE: usize = 0x4000;
pub const VRAM_SIZE: usize = 0x2000;
pub const OAM_SIZE: usize = 0xA0;
pub const IO_REG_SIZE: usize = 0x80;
pub const HRAM_SIZE: usize = 0x80;