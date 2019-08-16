use super::*;
use chrono::{Utc, DateTime, Timelike, Datelike};

const RAM_BANKS: usize = 8;
const ROM_BANKS: usize = 128;
const RTC_REG_SIZE: usize = 5;

pub struct MBC3 {
    pub ram: Vec<Byte>,
    pub rom: Vec<Byte>,
    ram_rtc_enabled: bool,
    rom_idx: u8,
    ram_idx: u8,
    // rtc_latch flag is used for detecting RTC 0x00 -> 0x01 write sequence
    rtc_latch: bool,
    pub rtc_reg: Vec<Byte>,
}

impl MBC3 {
    pub fn new(rom: Vec<Byte>) -> Self { 
        let mut mbc = Self {
            ram: vec![0; RAM_BANK_SIZE*RAM_BANKS],
            rom: vec![0; ROM_BANK_SIZE*ROM_BANKS],
            ram_rtc_enabled: true, rom_idx: 1, ram_idx: 0,
            rtc_latch: false, rtc_reg: vec![0; RTC_REG_SIZE],
        }; 
        if rom.len() > mbc.rom.len() { panic!("ROM too big for MBC1"); }
        for (i, byte) in rom.into_iter().enumerate() { mbc.rom[i] = byte; }
        mbc
    }

    fn datetime_to_rtc(&mut self, datetime: DateTime<Utc>) {
        self.rtc_reg[0] = datetime.second() as u8;
        self.rtc_reg[1] = datetime.minute() as u8;
        self.rtc_reg[2] = datetime.hour() as u8;
        
        let day = datetime.day() % (1 << 9);
        self.rtc_reg[3] = (day & 0xFF) as u8;
        self.rtc_reg[4] |= ((day & 0x0100) >> 8) as u8;
    }
}

impl BankController for MBC3 {
    fn get_addr_type(&self, addr: Addr) -> AddrType {
        let intervals = [
            (0x0000, 0x1FFF), // RAM RTC enable
            (0x2000, 0x3FFF), // ROM bank swap
            (0x4000, 0x5FFF), // RAM bank number / RTC register select
            (0x6000, 0x7FFF), // Latch clock data
        ];
        for (start, end) in intervals.iter() {
            if addr >= *start && addr <= *end { return AddrType::Status }
        }
        AddrType::Write
    }   

    fn on_status(&mut self, addr: Addr, value: Byte) {
        // RAM RTC enable, same as MBC1
        if addr < 0x2000 {
            self.ram_rtc_enabled = value & 0xF == 0xA;
        }

        // ROM bank select
        // All 7 bits used for bank selection.
        if addr >= 0x2000 && addr < 0x4000 {
            self.rom_idx = value & 0x7F;
            if self.rom_idx == 0 { self.rom_idx = 1; }
        }

        // Value in range 0x00-0x07 selects RAM idx.
        // Values in range 0x08-0x0C map RTC register to 0xA000-0xBFFF.
        if addr >= 0x4000 && addr < 0x6000 {
            // Selection is done in get_switchable_ram
            self.ram_idx = value;
        }

        // Latch Clock Data
        if addr >= 0x6000 && addr < 0x8000 {
            if value == 0x00 { self.rtc_latch = true; }
            else if value == 0x01 && self.rtc_latch {
                self.rtc_latch = false;
                // Flip HALT flag
                self.rtc_reg[4] ^= 0x80;
                // And update current register state.
                self.datetime_to_rtc(Utc::now());
            } else { self.rtc_latch = false; }
        }
    }

    fn get_base_rom(&mut self) -> Option<MutMem> { 
        Some(&mut self.rom[..ROM_BANK_SIZE]) 
    }

    fn get_switchable_rom(&mut self) -> Option<MutMem> {
        let start = (self.rom_idx as usize) * ROM_BANK_SIZE;
        let end = start + ROM_BANK_SIZE;
        Some(&mut self.rom[start..end])
    }

    fn get_switchable_ram(&mut self) -> Option<MutMem> {
        // When ram_idx points on RAM bank.
        if self.ram_idx <= 0x7 {
            let start = (self.ram_idx as usize) * RAM_BANK_SIZE;
            let end = start + RAM_BANK_SIZE;
            Some(&mut self.ram[start..end])
        } 
        // When ram_idx points to part of RTC register
        else {
            let halted = self.rtc_reg[4] & 0x80 != 0;
            if halted {
                let rtc_idx = (self.ram_idx - 8) as usize;
                Some(&mut self.rtc_reg[rtc_idx..rtc_idx+1])
            } else { None }
        }
    }
}
