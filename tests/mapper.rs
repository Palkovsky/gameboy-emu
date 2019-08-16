extern crate gameboy;

#[cfg(test)]
mod mbc {
    
    use gameboy::*;

    const SZ_32KB: usize = 1 << 15;
    const SZ_256KB: usize = 1 << 18;
    const SZ_2MB: usize = 1 << 21;

    fn mocked_rom(size: usize) -> Vec<u8> { 
        vec![0; size].into_iter().enumerate()
            .map(|(i, _)| (i % 256) as u8).collect() 
    }

    mod mbc2 {
        use super::*;

        #[test]
        #[should_panic]
        fn access_over_512_ram() {
            let mapper = mbc::MBC2::new(mocked_rom(SZ_256KB));
            let mut memory = mem::Memory::new(mapper);
            memory.read(RAM_SWITCHABLE_ADDR + 512);
        }

        #[test]
        #[should_panic]
        fn load_too_big_rom() {
            mbc::MBC2::new(mocked_rom(SZ_2MB));
        }

        #[test]
        #[should_panic]
        fn ram_access_when_disabled() {
            let mapper = mbc::MBC2::new(mocked_rom(SZ_256KB));
            let mut memory = mem::Memory::new(mapper);

            memory.write(0x0000, 0x00); // Disable RAM

            // RAM disabled -> should crash
            memory.write(RAM_SWITCHABLE_ADDR, 0xFF);
        }

        #[test]
        fn multiple_reads() {
            let mut mapper = mem::mbc::MBC2::new(mocked_rom(SZ_256KB));
            mapper.ram[128] = 0xFF;  
            mapper.ram[1] = 0x2E;
            mapper.rom[0x5*ROM_BANK_SIZE] = 0x11;
            mapper.rom[0x7*ROM_BANK_SIZE] = 0x22;  
            mapper.rom[0xF*ROM_BANK_SIZE+3] = 0x33;  
            let mut memory = mem::Memory::new(mapper);

            assert_eq!(memory.read(RAM_SWITCHABLE_ADDR + 128), 0x0F);
            assert_eq!(memory.read(RAM_SWITCHABLE_ADDR + 1), 0x0E);

            memory.write(0x2100, 0x5); // Select 5th ROM bank
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR), 0x11);

            memory.write(0x2300, 0xF); // Select 15th ROM bank
            assert_eq!(memory.read(ROM_SWITCHABLE_ADDR + 3), 0x33);
        }
    }

    mod mbc1 {
        use super::*;

        #[test]
        fn ram_enable_switch() {
            let mapper = mbc::MBC1::new(mocked_rom(SZ_2MB));
            let mut memory = mem::Memory::new(mapper);

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
            let mapper = mbc::MBC1::new(mocked_rom(SZ_2MB));
            let mut memory = mem::Memory::new(mapper);

            // Check default
            assert_eq!(memory.mapper.banking_mode, mbc::mbc1::ROM_MODE);

            // Enable RAM mode
            memory.write(0x6000, 0x01); 
            assert_eq!(memory.mapper.banking_mode, mbc::mbc1::RAM_MODE);
        }

        #[test]
        fn ram_access_in_rom_mode() {
            let mut mapper = mbc::MBC1::new(mocked_rom(SZ_2MB));
            mapper.ram[0] = 0x21; // Firt RAM bank
            mapper.ram[RAM_BANK_SIZE] = 0x37; // Second RAM bank
            let mut memory = mem::Memory::new(mapper);

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
        fn multiple_reads() {
            let mut mapper = mbc::MBC1::new(mocked_rom(SZ_2MB));
            mapper.ram[3*RAM_BANK_SIZE] = 0x69;  
            mapper.ram[2*RAM_BANK_SIZE+1] = 0x70;
            mapper.rom[21*ROM_BANK_SIZE] = 0x11;
            mapper.rom[66*ROM_BANK_SIZE] = 0x22;  
            mapper.rom[88*ROM_BANK_SIZE+3] = 0x33;
            let mut memory = mem::Memory::new(mapper);

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

    mod rom_only {
        use super::*;

        #[test]
        fn read() {
            let rom = mocked_rom(SZ_32KB);
            let mut mapper = mbc::RomOnly::new(rom.clone());
            mapper.ram[0x0069] = 0x69;
            let mut memory = Memory::new(mapper);
            
            // Read from ROM
            assert_eq!(memory.read(ROM_BASE_ADDR + 0x0), rom[0x0]);
            assert_eq!(memory.read(ROM_BASE_ADDR + 0x2137),rom[0x2137]);
            assert_eq!(memory.read(ROM_BASE_ADDR + 0x7FFF), rom[0x7FFF]);
            // Read from RAM
            assert_eq!(memory.read(RAM_BASE_ADDR + 0x0069), 0x69);
            assert_eq!(memory.read(RAM_BASE_ADDR + 0x006A), 0x00);
        }

        #[test]
        #[should_panic]
        fn write_rom() {
            let mapper = mbc::RomOnly::new(mocked_rom(SZ_32KB));
            let mut memory = Memory::new(mapper);

            // Writing to ROM segment -> should panic
            memory.write(0x2137, 0x69);
        }

        #[test]
        fn write_ram() {
            let mapper = mbc::RomOnly::new(mocked_rom(SZ_32KB));
            let mut memory = Memory::new(mapper);

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
            let mapper = mbc::RomOnly::new(mocked_rom(SZ_32KB));
            let mut memory = Memory::new(mapper);

            // Reading switchable RAM -> Rom only doesn't support it
            memory.read(RAM_SWITCHABLE_ADDR as u16);
        }
    }
}