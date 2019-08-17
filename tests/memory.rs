#[cfg(test)]
mod memory {
    use gameboy::*;

    const SZ_2MB: usize = 1 << 21;
    fn mock_memory(rom_size: usize) -> Memory<mbc::MBC1> {
        let mapper = mbc::MBC1::new(vec![0; rom_size]);
        mem::Memory::new(mapper)
    }

    mod boot {
        use super::*;

        #[test]
        #[should_panic]
        fn write_to_bootstrap() {
            let mut memory = mock_memory(SZ_2MB);
            memory.map_bootsrap();
            memory.write(0x0000, 0x21);
        }

        #[test]
        fn map_unmap() {
            let mut memory = mock_memory(SZ_2MB);
            memory.map_bootsrap();

            // Check first bytes of bootsrap
            assert_eq!(memory.read(0), 0x31);
            assert_eq!(memory.read(1), 0xFE);
            assert_eq!(memory.read(16), 0x11);
            assert_eq!(memory.read(0xA0), 0x05);
            assert_eq!(memory.read(255), 0x50);

            memory.unmap_bootsrap();
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

            let vram = memory.gpu.vram();
            assert_eq!(vram[0], 0x01);
            assert_eq!(vram[0x69], 0x21);
            assert_eq!(vram[vram.len()-1], 0x37);
        }

        #[test]
        fn vram_read() {
            let mut memory = mock_memory(SZ_2MB);

            let vram = memory.gpu.vram();
            vram[0] = 0x1;
            vram[0x69] = 0x21;
            vram[vram.len() - 1] = 0x37;

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

            let oam = memory.gpu.oam();
            assert_eq!(oam[0], 0x01);
            assert_eq!(oam[0x69], 0x21);
            assert_eq!(oam[oam.len()-1], 0x37);
        }

        #[test]
        fn oam_read() {
            let mut memory = mock_memory(SZ_2MB);

            let oam = memory.gpu.oam();
            oam[0] = 0x1;
            oam[0x69] = 0x21;
            oam[oam.len() - 1] = 0x37;

            assert_eq!(memory.read(OAM_ADDR), 0x01);
            assert_eq!(memory.read(OAM_ADDR + 0x69), 0x21);
            assert_eq!(memory.read(OAM_ADDR + OAM_SIZE as u16 - 1), 0x37);
        }
    }

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
            assert_eq!(memory.read(LCDC), 0x91 | 0x02);

            let ie = memory.read(IE);
            memory.write(IE, ie | 0x0F);
            assert_eq!(memory.read(IE), 0x0F);
        }
    }
}