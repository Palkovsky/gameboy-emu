extern crate gameboy;

#[cfg(test)]
mod memtest {
    use gameboy::*;

    const SZ_2MB: usize = 1 << 21;
    
    fn mock_memory(rom_size: usize) -> MMU<mbc::MBC1> {
        let mapper = mbc::MBC1::new(vec![0; rom_size]);
        mem::MMU::new(mapper)
    }

    #[cfg(test)]
    mod boot {
        use super::*;

        #[test]
        #[should_panic]
        fn write_to_bootstrap() {
            let mut memory = mock_memory(SZ_2MB);
            memory.write(BOOT_END, 0);
            memory.write(0x0000, 0x21);
        }

        #[test]
        fn map_unmap() {
            let mut memory = mock_memory(SZ_2MB);
            memory.write(BOOT_END, 0);

            // Check first bytes of bootsrap
            assert_eq!(memory.read(0), 0x31);
            assert_eq!(memory.read(1), 0xFE);
            assert_eq!(memory.read(16), 0x11);
            assert_eq!(memory.read(0xA0), 0x05);
            assert_eq!(memory.read(255), 0x50);

            memory.write(BOOT_END, 1);
            assert_eq!(memory.read(0), 0);
            assert_eq!(memory.read(1), 0);
            assert_eq!(memory.read(16), 0);
            assert_eq!(memory.read(0xA0), 0);
            assert_eq!(memory.read(255), 0);
        }
    }

    mod gpu {
        use super::*;

        #[test]
        fn vram_write() {
            let mut memory = mock_memory(SZ_2MB);

            memory.write(VRAM_ADDR, 0x1);
            memory.write(VRAM_ADDR + 0x69, 0x21);
            memory.write(VRAM_ADDR + VRAM_SIZE as u16 - 1, 0x37);

            assert_eq!(memory.vram[0], 0x01);
            assert_eq!(memory.vram[0x69], 0x21);
            assert_eq!(memory.vram[memory.vram.len()-1], 0x37);
        }

        #[test]
        fn vram_read() {
            let mut memory = mock_memory(SZ_2MB);
            let len = memory.vram.len();

            memory.vram[0] = 0x1;
            memory.vram[0x69] = 0x21;
            memory.vram[len - 1] = 0x37;

            assert_eq!(memory.read(VRAM_ADDR), 0x01);
            assert_eq!(memory.read(VRAM_ADDR + 0x69), 0x21);
            assert_eq!(memory.read(VRAM_ADDR + VRAM_SIZE as u16 - 1), 0x37);
        }

        #[test]
        fn oam_write() {
            let mut memory = mock_memory(SZ_2MB);

            memory.write(OAM_ADDR, 0x1);
            memory.write(OAM_ADDR + 0x69, 0x21);
            memory.write(OAM_ADDR + OAM_SIZE as u16 - 1, 0x37);

            assert_eq!(memory.oam[0], 0x01);
            assert_eq!(memory.oam[0x69], 0x21);
            assert_eq!(memory.oam[memory.oam.len()-1], 0x37);
        }

        #[test]
        fn oam_read() {
            let mut memory = mock_memory(SZ_2MB);
            let len = memory.oam.len();

            memory.oam[0] = 0x1;
            memory.oam[0x69] = 0x21;
            memory.oam[len - 1] = 0x37;

            assert_eq!(memory.read(OAM_ADDR), 0x01);
            assert_eq!(memory.read(OAM_ADDR + 0x69), 0x21);
            assert_eq!(memory.read(OAM_ADDR + OAM_SIZE as u16 - 1), 0x37);
        }
    }

    #[cfg(test)]
    mod ioregs {
        use super::*;

        #[test]
        fn io_defaults() {
            let mut memory = mock_memory(SZ_2MB);

            // Checks if defaults are OK
            assert_eq!(memory.read(P1), 0x00);
            assert_eq!(memory.read(LCDC), 0x91);
            assert_eq!(memory.read(NR_10), 0x80);
            assert_eq!(memory.read(OBP_0), 0xFF);
            assert_eq!(memory.read(IE), 0x00);
        }

        
        #[test]
        fn io_read_write() {
            let mut memory = mock_memory(SZ_2MB);

            let lcdc = memory.read(LCDC);
            memory.write(LCDC, lcdc | 0x02);
            assert_eq!(memory.read(mem::ioregs::LCDC), 0x91 | 0x02);

            let ie = memory.read(IE);
            memory.write(IE, ie | 0x0F);
            assert_eq!(memory.read(IE), 0x0F);
        }
    }
}