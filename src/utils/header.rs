use std::fmt::{Formatter, Result, Display};
use std::str;

use super::super::{ROM_BANK_SIZE, RAM_BANK_SIZE};

/* Data stored in cart ROM at 0x100-0x14F */
#[repr(packed)]
pub struct CartHeader {
    entrypoint: [u8; 4],
    logo: [u8; 48],
    title: [u8; 16],
    license_new: [u8; 2],
    sgb: u8,
    cart_type: u8,
    rom_size: u8,
    ram_size: u8,
    destination: u8,
    license_old: u8,
    version: u8,
    header_checksum: u8,
    global_checksum: [u8; 2],
}

#[derive(Debug)]
pub enum CartType {
    RomOnly(),
    Mbc1(), Mbc2(), Mbc3(),
    Unknown(u8),
}

impl CartHeader {
    pub fn new(rom: Vec<u8>) -> Self {
        let bytes = std::mem::size_of::<CartHeader>();
        if bytes != rom.len() {
            panic!("Cart header must be {} bytes long, but provided bytes are {} bytes long.", bytes, rom.len());
        }
        unsafe { std::ptr::read(rom.as_ptr() as *const _) }
    }

    pub fn title(&self) -> String {
        let slice = str::from_utf8(if self.license_old == 0x33 {
                &self.title[..11] 
            } else {
                &self.title[..16] 
            }
        ).unwrap();
        String::from(slice)
    }

    pub fn license(&self) -> u8 {
        if self.license_old != 0x33 { 
            self.license_old 
        } else { 
            let string = str::from_utf8(&self.license_new).unwrap();
            u8::from_str_radix(string, 16).unwrap()
        }
    }

    pub fn sgb_support(&self) -> bool {
        self.sgb == 0x003
    }

    pub fn cart_type(&self) -> CartType {
        match self.cart_type {
            0x00 | 0x08 | 0x09 => CartType::RomOnly(),
            0x01 | 0x02 | 0x03 => CartType::Mbc1(),
            0x05 | 0x06 => CartType::Mbc2(),
            0x0F | 0x10 | 0x11 | 0x12 | 0x13 => CartType::Mbc3(),
            other => CartType::Unknown(other),
        }
    }

    pub fn rom_size(&self) -> usize {
        // Calculated as 32KB shl N
        ((1 << 15) << self.rom_size) as usize
    }

    pub fn rom_banks(&self) -> usize {
        self.rom_size() / ROM_BANK_SIZE
    }

    pub fn ram_size(&self) -> usize {
        match self.ram_size {
            0x00 => 0, 
            0x01 => 1 << 11, // 2KB
            0x02 => 1 << 13, // 8KB
            0x03 => 1 << 15, // 32KB
            0x04 => 1 << 17, // 128KB
            0x05 => 1 << 16, // 64KB
            _ => panic!("Invalid RAM size: {}", self.ram_size)
        }
    }

    pub fn ram_banks(&self) -> usize {
        self.ram_size() / RAM_BANK_SIZE
    }

    pub fn is_japan(&self) -> bool {
        self.destination == 0x00
    }

    pub fn checksum(&self) -> u8 {
        self.header_checksum
    }
}

impl Display for CartHeader {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, 
              "(Title: {}, MBC: {:?}, ROM banks: {}, RAM banks: {}, SGB: {}, Japanese: {})",
              self.title(), self.cart_type(), self.rom_banks(), self.ram_banks(), self.sgb_support(), self.is_japan())
    }
}