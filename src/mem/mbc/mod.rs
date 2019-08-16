pub mod romonly;
pub mod mbc1;
pub mod mbc2;
pub mod mbc3;

pub use mbc1::{MBC1};
pub use mbc2::{MBC2};
pub use mbc3::{MBC3};
pub use romonly::{RomOnly};

use super::{ROM_BANK_SIZE, RAM_BANK_SIZE, Addr, Byte, MutMem};


/*
 * AddrType is used by BankController to determine address type: wheater it is
 * will change MBC registers or perform bank switching or is just regular writable.
 */
#[derive(Copy, Clone)]
pub enum AddrType {
    Write,
    Status,
}
/*
 * BankController trait represents memory mapper interface.
 */
pub trait BankController {
    /*
     * Checks whether the addr is special memory region for
     * MBC configuration(setting registers, enabling RAM etc.). 
     */
    fn get_addr_type(&self, addr: Addr) -> AddrType;
    /* Called when get_addr_type() returned Status addr type. */
    fn on_status(&mut self, addr: Addr, value: Byte);
    /* Gets base non-switchable ROM. 0x0000-0x4000 range */
    fn get_base_rom(&mut self) -> Option<MutMem>;
    /* Gets switchable ROM. 0x4000-0x8000 range */
    fn get_switchable_rom(&mut self) -> Option<MutMem>;
    /* Gets switchable RAM. 0xA000-0xC000 range */
    fn get_switchable_ram(&mut self) -> Option<MutMem>;
}