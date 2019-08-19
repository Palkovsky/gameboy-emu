extern crate gameboy;

#[cfg(test)]
mod gputest {
    use gameboy::*;

    fn mock() -> (MMU<mbc::MBC1>, GPU) {
        let mapper = mbc::MBC1::new(vec![0; 1 << 21]);
        (mem::MMU::new(mapper), GPU::new())
    }

    #[test]
    fn ly_updates() {
        let (mut mmu, mut gpu) = mock();

        // 60 frames
        for _ in 0..60 {
            for ly in 0..(gpu::SCREEN_HEIGHT + gpu::VBLANK_HEIGHT) {
                assert_eq!(mmu.read(ioregs::LY), ly as u8);
                for _ in 0..gpu::SCANLINE_CYCLES { gpu.step(&mut mmu); }
            }
        }
    }

    #[test]
    fn mode_changes() {
        let (mut mmu, mut gpu) = mock();

        // Run 60 frames
        for _ in 0..60 {

            // Check if OAM/LCD/HBLANK states take proper number of cycles
            for _ in 0..gpu::SCREEN_HEIGHT {
                for _ in 0..gpu::OAM_SEARCH_CYCLES {
                    assert_eq!(gpu.MODE, gpu::GPUMode::OAM_SEARCH);
                    gpu.step(&mut mmu);
                }

                for _ in 0..gpu::LCD_TRANSFER_CYCLES {
                    assert_eq!(gpu.MODE, gpu::GPUMode::LCD_TRANSFER);
                    gpu.step(&mut mmu);
                }

                for _ in 0..gpu::HBLANK_CYCLES {
                    assert_eq!(gpu.MODE, gpu::GPUMode::HBLANK);
                    gpu.step(&mut mmu);
                }
            }

            // Check if VBLANK takes proper number of cycles
            for _ in 0..gpu::VBLANK_CYCLES {
                assert_eq!(gpu.MODE, gpu::GPUMode::VBLANK);
                gpu.step(&mut mmu);
            }
        }
    }

    #[test]
    fn flag_updates() {
        let (mut mmu, mut gpu) = mock();

        mmu.write(ioregs::LCDC, 0b10010001);
        gpu.step(&mut mmu);

        assert_eq!(gpu.LCD_DISPLAY_ENABLE, true);
        assert_eq!(gpu.WINDOW_TILE_MAP_SELECT, false);
        assert_eq!(gpu.WINDOW_DISPLAY_ENABLE, false);
        assert_eq!(gpu.BG_WINDOW_TILE_DATA_SELECT, true);
        assert_eq!(gpu.BG_TILE_MAP_DISPLAY_SELECT, false);
        assert_eq!(gpu.SPRITE_SIZE, false);
        assert_eq!(gpu.SPRITE_DISPLAY_ENABLE, false);
        assert_eq!(gpu.DISPLAY_PRIORITY, true);

        mmu.write(ioregs::STAT, 0b10010000);
        gpu.step(&mut mmu);

        assert_eq!(gpu.COINCIDENCE_INTERRUPT_ENABLE, false);
        assert_eq!(gpu.MODE_2_OAM_INTERRUPT_ENABLE, false);
        assert_eq!(gpu.MODE_1_VBLANK_INTERRUPT_ENABLE, true);
        assert_eq!(gpu.MODE_0_HBLANK_INTERRUPT_ENABLE, false);
        assert_eq!(gpu.COINCIDENCE_FLAG, true);
        assert_eq!(gpu.MODE, gpu::GPUMode::OAM_SEARCH);
    }
}