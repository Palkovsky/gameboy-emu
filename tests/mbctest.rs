extern crate gameboy;

#[cfg(test)]
mod mbctest {
    use gameboy::*;

    const SZ_32KB: usize = 1 << 15;
    const SZ_256KB: usize = 1 << 18;
    const SZ_2MB: usize = 1 << 21;

    fn mock_rom(size: usize) -> Vec<u8> { 
        vec![0; size].into_iter().enumerate()
            .map(|(i, _)| (i % 256) as u8).collect() 
    }
    
    fn mock_romonly() -> mbc::RomOnly { mbc::RomOnly::new(mock_rom(SZ_32KB)) }
    fn mock_mbc1() -> mbc::MBC1 { mbc::MBC1::new(mock_rom(SZ_2MB)) }
    fn mock_mbc2() -> mbc::MBC2 { mbc::MBC2::new(mock_rom(SZ_256KB)) }
    fn mock_mbc3() -> mbc::MBC3 { mbc::MBC3::new(mock_rom(SZ_2MB)) }

    fn mock_memory<T: mbc::BankController>(mapper: T) -> MMU<T> {
        let mut mem = mem::MMU::new(mapper);
        mem.write(BOOT_END, 1); // Disable bootstrap mapping
        mem
    }

    #[cfg(test)]
    mod mbc3 {
        use super::*;
        use chrono::{Utc, Timelike, Datelike};

         #[test]
        fn access_0h_20h_40h_60h_bank() {
            let mut mem = mock_memory(mock_mbc3());
            mem.mapper.rom[ROM_BANK_SIZE * 0x00] = 0x37;
            mem.mapper.rom[ROM_BANK_SIZE * 0x01] = 0x01;
            mem.mapper.rom[ROM_BANK_SIZE * 0x20] = 0x20;
            mem.mapper.rom[ROM_BANK_SIZE * 0x21] = 0x21;
            mem.mapper.rom[ROM_BANK_SIZE * 0x40] = 0x40;
            mem.mapper.rom[ROM_BANK_SIZE * 0x41] = 0x41;
            mem.mapper.rom[ROM_BANK_SIZE * 0x60] = 0x60;
            mem.mapper.rom[ROM_BANK_SIZE * 0x61] = 0x61;

            // Try selecting 0h memory bank
            mem.write(0x2000, 0);
            assert_eq!(mem.read(ROM_SWITCHABLE_ADDR), 0x1);
            assert_eq!(mem.read(ROM_BASE_ADDR), 0x37);

            // Try selecting 20h memory bank
            mem.write(0x2000, 0x20);
            assert_eq!(mem.read(ROM_SWITCHABLE_ADDR), 0x20);

            // Try selecting 40h memory bank
            mem.write(0x2000, 0x40);
            assert_eq!(mem.read(ROM_SWITCHABLE_ADDR), 0x40);

            // Try selecting 60h memory bank
            mem.write(0x2000, 0x60);
            assert_eq!(mem.read(ROM_SWITCHABLE_ADDR), 0x60);
        }

        #[test]
        fn ram_read() {
            let mut mem = mock_memory(mock_mbc3());
            mem.mapper.ram[RAM_BANK_SIZE * 0x00 + 11] = 0x01;
            mem.mapper.ram[RAM_BANK_SIZE * 0x04 + 44] = 0x04;
            mem.mapper.ram[RAM_BANK_SIZE * 0x07 + 77] = 0x07;

            // RAM bank #0 should be selected by default
            assert_eq!(mem.read(RAM_SWITCHABLE_ADDR + 11), 0x01);

            // Switch to bank #7
            mem.write(0x4000, 0x07);
            assert_eq!(mem.read(RAM_SWITCHABLE_ADDR + 77), 0x07);

            // Switch to bank #4
            mem.write(0x4000, 0x04);
            assert_eq!(mem.read(RAM_SWITCHABLE_ADDR + 44), 0x04);
        }

        #[test]
        fn rtc_read() {
            let mut mem = mock_memory(mock_mbc3());

            // Shouldn't be halted
            assert!(mem.mapper.rtc_reg[4] & 0x80 == 0);
            // Latch current RTC state
            mem.write(0x6000, 0x00);
            // Shouldn't be halted
            assert!(mem.mapper.rtc_reg[4] & 0x80 == 0);
            // Finsh latch sequence
            mem.write(0x6000, 0x01);
            // Should be halted
            assert!(mem.mapper.rtc_reg[4] & 0x80 != 0);

            let time = Utc::now();
            
            // Map RTC seconds to 0xA000
            mem.write(0x4000, 0x8);
            assert_eq!(time.second() as u8, mem.read(RAM_SWITCHABLE_ADDR));
            // Map RTC mins to 0xA000
            mem.write(0x4000, 0x9);
            assert_eq!(time.minute() as u8, mem.read(RAM_SWITCHABLE_ADDR));
            // Map RTC hours to 0xA000
            mem.write(0x4000, 0xA);
            assert_eq!(time.hour() as u8, mem.read(RAM_SWITCHABLE_ADDR));
            // Map RTC day lower 8 bits to 0xA000
            let day = time.day() % (1 << 9);
            mem.write(0x4000, 0xB);
            assert_eq!((day & 0xFF) as u8, mem.read(RAM_SWITCHABLE_ADDR));
            // Map last RTC byte 0xA000
            mem.write(0x4000, 0xC);
            let byte = mem.read(RAM_SWITCHABLE_ADDR);
            assert!(byte & 0x80 != 0);
            assert_eq!(((day & 0x0100) >> 8) as u8, byte & 1);
        }

        #[test]
        #[should_panic]
        fn rtc_read_not_latched() {
            let mut memory = mock_memory(mock_mbc3());
            memory.write(0x4000, 0x8);
            memory.read(RAM_SWITCHABLE_ADDR);
        }

        #[test]
        fn rtc_latching() {
            let mut memory = mock_memory(mock_mbc3());
            // Shouldn't be halted
            assert!(memory.mapper.rtc_reg[4] & 0x80 == 0);
            // Latch current RTC state
            memory.write(0x6000, 0x00);
            // Still halted
            assert!(memory.mapper.rtc_reg[4] & 0x80 == 0);
            // Finish latch sequence
            memory.write(0x6000, 0x01);
            // Should be halted
            assert!(memory.mapper.rtc_reg[4] & 0x80 != 0);
            // Unlatch current RTC state
            memory.write(0x6000, 0x00);
            // Should be still halted
            assert!(memory.mapper.rtc_reg[4] & 0x80 != 0);
            // Finish unlatching
            memory.write(0x6000, 0x01);
            // Should be unhalted now
            assert!(memory.mapper.rtc_reg[4] & 0x80 == 0);
        }
    }

    #[cfg(test)]
    mod mbc2 {
        use super::*;

        #[test]
        #[should_panic]
        fn access_over_512_ram() {
            let mut memory = mock_memory(mock_mbc2());
            memory.read(RAM_SWITCHABLE_ADDR + 512);
        }

        #[test]
        #[should_panic]
        fn load_too_big_rom() {
            mbc::MBC2::new(mock_rom(SZ_2MB));
        }

        #[test]
        #[should_panic]
        fn ram_access_when_disabled() {
            let mut memory = mock_memory(mock_mbc2());

            memory.write(0x0000, 0x00); // Disable RAM

            // RAM disabled -> should crash
            memory.write(RAM_SWITCHABLE_ADDR, 0xFF);
        }

        #[test]
        fn multiple_reads() {
            let mut memory = mock_memory(mock_mbc2());
            memory.mapper.ram[128] = 0xFF;  
            memory.mapper.ram[1] = 0x2E;
            memory.mapper.rom[0x5*ROM_BANK_SIZE] = 0x11;
            memory.mapper.rom[0x7*ROM_BANK_SIZE] = 0x22;  
            memory.mapper.rom[0xF*ROM_BANK_SIZE+3] = 0x33;  

            assert_eq!(memory.read(RAM_SWITCHABLE_ADDR + 128), 0x0F);
            assert_eq!(memory.read(RAM_SWITCHABLE_ADDR + 1), 0x0E);

            memory.write(0x2100, 0x5); // Select 5th ROM bank
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR), 0x11);

            memory.write(0x2300, 0xF); // Select 15th ROM bank
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR + 3), 0x33);
        }
    }

    #[cfg(test)]
    mod mbc1 {
        use super::*;

        #[test]
        fn ram_enable_switch() {
            let mut memory = mock_memory(mock_mbc1());

            // Check default
            assert_eq!(memory.mapper.ram_enabled, true);

            // Disable RAM
            memory.write(0x0000, 0x00); 
            assert_eq!(memory.mapper.ram_enabled, false);

            // Trying to enable RAM with invalid bit sequence
            memory.write(0x0000, 0x0B);
            assert_eq!(memory.mapper.ram_enabled, false);

            // Enable RAM with valid bit sequence
            memory.write(0x0000, 0x0A);
            assert_eq!(memory.mapper.ram_enabled, true);
        }

        #[test]
        fn ram_rom_mode_switch() {
            let mut memory = mock_memory(mock_mbc1());

            // Check default
            assert_eq!(memory.mapper.banking_mode, mbc::mbc1::ROM_MODE);

            // Enable RAM mode
            memory.write(0x6000, 0x01); 
            assert_eq!(memory.mapper.banking_mode, mbc::mbc1::RAM_MODE);
        }

        #[test]
        fn ram_access_in_rom_mode() {
            let mut memory = mock_memory(mock_mbc1());
            memory.mapper.ram[0] = 0x21; // Firt RAM bank
            memory.mapper.ram[RAM_BANK_SIZE] = 0x37; // Second RAM bank

            // Check if in ROM mode
            assert_eq!(memory.mapper.banking_mode, mbc::mbc1::ROM_MODE);

            // Switch RAM bank to 0x01
            memory.write(0x4000, 0x01);

            // Since it's in ROM mode it shouldn't really change bank ans serve bank 0x00
            assert_eq!(memory.read(RAM_SWITCHABLE_ADDR), 0x21);

            // Switch to RAM mode
            memory.write(0x6000, 0x01);

            // Now the change should be visible
            assert_eq!(memory.read(RAM_SWITCHABLE_ADDR), 0x37);
        }

        #[test]
        fn access_0h_20h_40h_60h_bank() {
            let mut memory = mock_memory(mock_mbc1());
            memory.mapper.rom[ROM_BANK_SIZE * 0x00] = 0x37;
            memory.mapper.rom[ROM_BANK_SIZE * 0x01] = 0x01;
            memory.mapper.rom[ROM_BANK_SIZE * 0x20] = 0x20;
            memory.mapper.rom[ROM_BANK_SIZE * 0x21] = 0x21;
            memory.mapper.rom[ROM_BANK_SIZE * 0x40] = 0x40;
            memory.mapper.rom[ROM_BANK_SIZE * 0x41] = 0x41;
            memory.mapper.rom[ROM_BANK_SIZE * 0x60] = 0x60;
            memory.mapper.rom[ROM_BANK_SIZE * 0x61] = 0x61;

            // Check if in ROM mode
            assert_eq!(memory.mapper.banking_mode, mbc::mbc1::ROM_MODE);

            // Try selecting 0h memory bank
            memory.write(0x4000, 0);
            memory.write(0x2000, 0);
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR), 0x1);
            assert_eq!(memory.read(ROM_BASE_ADDR), 0x37);

            // Try selecting 20h memory bank
            memory.write(0x2000, 0x0000);
            memory.write(0x4000, 0b00000001);
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR), 0x21);

            // Try selecting 40h memory bank
            memory.write(0x2000, 0x0000);
            memory.write(0x4000, 0b00000010);
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR), 0x41);

            // Try selecting 60h memory bank
            memory.write(0x2000, 0x0000);
            memory.write(0x4000, 0b00000011);
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR), 0x61);
        }

        #[test]
        fn multiple_reads() {
            let mut memory = mock_memory(mock_mbc1());
            memory.mapper.ram[3*RAM_BANK_SIZE] = 0x69;  
            memory.mapper.ram[2*RAM_BANK_SIZE+1] = 0x70;
            memory.mapper.rom[21*ROM_BANK_SIZE] = 0x11;
            memory.mapper.rom[66*ROM_BANK_SIZE] = 0x22;  
            memory.mapper.rom[88*ROM_BANK_SIZE+3] = 0x33;

            memory.write(0x0000, 0x0A); // Enable RAM
            memory.write(0x6000, 1); // Enable 4 RAM banks mode

            memory.write(0x4000, 0x3);  // Select 3rd RAM bank
            assert_eq!(memory.read(RAM_SWITCHABLE_ADDR), 0x69);

            memory.write(0x4000, 0x2);  // Select 2nd RAM bank
            assert_eq!(memory.read(RAM_SWITCHABLE_ADDR + 1), 0x70);

            memory.write(0x2000, 21); // Select 21st ROM bank
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR), 0x11);

            memory.write(0x2000, 66); // Select 66th ROM bank
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR), 0x00);

            memory.write(0x6000, 0); // Enable 1 RAM bank mode
            memory.write(0x2000, 66); // Select 66th ROM bank
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR), 0x22);

            memory.write(0x2000, 88); // Select 88th ROM bank
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR + 3), 0x33);
        }
    }

    #[cfg(test)]
    mod rom_only {
        use super::*;

        #[test]
        fn read() {
            let mut memory = mock_memory(mock_romonly());

            // Read from ROM
            assert_eq!(memory.read(ROM_BASE_ADDR + 0x0),  memory.mapper.rom[0x0]);
            assert_eq!(memory.read(ROM_BASE_ADDR + 0x2137), memory.mapper.rom[0x2137]);
            assert_eq!(memory.read(ROM_BASE_ADDR + 0x7FFF), memory.mapper.rom[0x7FFF]);

            // Read from RAM
            memory.write(RAM_BASE_ADDR, 0x69);
            assert_eq!(memory.read(RAM_BASE_ADDR), 0x69);
            assert_eq!(memory.read(RAM_BASE_ADDR + 1), 0x00);
        }

        #[test]
        #[should_panic]
        fn write_rom() {
            let mut memory = mock_memory(mock_romonly());

            // Writing to ROM segment -> should panic
            memory.write(0x2137, 0x69);
        }

        #[test]
        fn write_ram() {
            let mut memory = mock_memory(mock_romonly());

            memory.write(RAM_BASE_ADDR + 0x69, 0x21);
            memory.write(RAM_BASE_ADDR + 0x69, 0x37);
            memory.write(RAM_BASE_ADDR + 0x6A, 0x37);

            assert_eq!(memory.read(RAM_BASE_ADDR + 0x69), 0x37);
            assert_eq!(memory.read(RAM_BASE_ADDR + 0x6A), 0x37);
            assert_eq!(memory.read(RAM_BASE_ADDR + 0x6B), 0x00);
        }

        #[test]
        #[should_panic]
        fn accessing_switchable_ram() {
            let mut memory = mock_memory(mock_romonly());

            // Reading switchable RAM -> Rom only doesn't support it
            memory.read(RAM_SWITCHABLE_ADDR as u16);
        }
    }
}