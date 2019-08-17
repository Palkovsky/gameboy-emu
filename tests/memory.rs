#[cfg(test)]
mod memory {
    use gameboy::*;

    const SZ_2MB: usize = 1 << 21;
    fn mock_memory(rom_size: usize) -> Memory<mbc::MBC1> {
        let mapper = mbc::MBC1::new(vec![0; rom_size]);
        mem::Memory::new(mapper)
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