extern crate gameboy;

#[cfg(test)]
mod memtest {
    use gameboy::*;

    const SZ_2MB: usize = 1 << 21;
    
    fn gen_mmu(rom_size: usize) -> MMU<mbc::MBC1> {
        let mapper = mbc::MBC1::new(vec![0; rom_size]);
        mem::MMU::new(mapper)
    }

    #[cfg(test)]
    mod boot {
        use super::*;

        #[test]
        #[should_panic]
        fn write_to_bootstrap() {
            let mut mmu = gen_mmu(SZ_2MB);
            mmu.write(BOOT, 0);
            mmu.write(0x0000, 0x21);
        }

        #[test]
        fn map_unmap() {
            let mut mmu = gen_mmu(SZ_2MB);
            mmu.write(BOOT, 0);

            // Check first bytes of bootsrap
            assert_eq!(mmu.read(0), 0x31);
            assert_eq!(mmu.read(1), 0xFE);
            assert_eq!(mmu.read(16), 0x11);
            assert_eq!(mmu.read(0xA0), 0x05);
            assert_eq!(mmu.read(255), 0x50);

            mmu.write(BOOT, 1);
            assert_eq!(mmu.read(0), 0);
            assert_eq!(mmu.read(1), 0);
            assert_eq!(mmu.read(16), 0);
            assert_eq!(mmu.read(0xA0), 0);
            assert_eq!(mmu.read(255), 0);
        }
    }

    mod gpu {
        use super::*;

        #[test]
        fn vram_write() {
            let mut mmu = gen_mmu(SZ_2MB);

            mmu.write(VRAM_ADDR, 0x1);
            mmu.write(VRAM_ADDR + 0x69, 0x21);
            mmu.write(VRAM_ADDR + VRAM_SIZE as u16 - 1, 0x37);

            assert_eq!(mmu.vram[0], 0x01);
            assert_eq!(mmu.vram[0x69], 0x21);
            assert_eq!(mmu.vram[mmu.vram.len()-1], 0x37);
        }

        #[test]
        fn vram_read() {
            let mut mmu = gen_mmu(SZ_2MB);
            let len = mmu.vram.len();

            mmu.vram[0] = 0x1;
            mmu.vram[0x69] = 0x21;
            mmu.vram[len - 1] = 0x37;

            assert_eq!(mmu.read(VRAM_ADDR), 0x01);
            assert_eq!(mmu.read(VRAM_ADDR + 0x69), 0x21);
            assert_eq!(mmu.read(VRAM_ADDR + VRAM_SIZE as u16 - 1), 0x37);
        }

        #[test]
        fn oam_write() {
            let mut mmu = gen_mmu(SZ_2MB);

            mmu.write(OAM_ADDR, 0x1);
            mmu.write(OAM_ADDR + 0x69, 0x21);
            mmu.write(OAM_ADDR + OAM_SIZE as u16 - 1, 0x37);

            assert_eq!(mmu.oam[0], 0x01);
            assert_eq!(mmu.oam[0x69], 0x21);
            assert_eq!(mmu.oam[mmu.oam.len()-1], 0x37);
        }

        #[test]
        fn oam_read() {
            let mut mmu = gen_mmu(SZ_2MB);
            let len = mmu.oam.len();

            mmu.oam[0] = 0x1;
            mmu.oam[0x69] = 0x21;
            mmu.oam[len - 1] = 0x37;

            assert_eq!(mmu.read(OAM_ADDR), 0x01);
            assert_eq!(mmu.read(OAM_ADDR + 0x69), 0x21);
            assert_eq!(mmu.read(OAM_ADDR + OAM_SIZE as u16 - 1), 0x37);
        }
    }

    #[cfg(test)]
    mod ioregs {
        use super::*;

        #[test]
        fn io_defaults() {
            let mut mmu = gen_mmu(SZ_2MB);

            // Checks if defaults are OK
            assert_eq!(mmu.read(P1), 0x00);
            assert_eq!(mmu.read(LCDC), 0x91);
            assert_eq!(mmu.read(NR_10), 0x80);
            assert_eq!(mmu.read(OBP_0), 0xFF);
            assert_eq!(mmu.read(IE), 0x00);
        }

        
        #[test]
        fn io_read_write() {
            let mut mmu = gen_mmu(SZ_2MB);

            let lcdc = mmu.read(LCDC);
            mmu.write(LCDC, lcdc | 0x02);
            assert_eq!(mmu.read(mem::ioregs::LCDC), 0x91 | 0x02);

            let ie = mmu.read(IE);
            mmu.write(IE, ie | 0x0F);
            assert_eq!(mmu.read(IE), 0x0F);
        }
    }
}