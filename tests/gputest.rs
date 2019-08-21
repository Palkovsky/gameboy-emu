extern crate gameboy;

#[cfg(test)]
mod gputest {
    use gameboy::*;

    fn mock() -> (MMU<mbc::MBC1>, GPU) {
        (mem::MMU::new(mbc::MBC1::new(vec![0; 1 << 21])), GPU::new())
    }

    fn mock_state() -> State<mbc::MBC1> {
        State::new(MMU::new(MBC1::new(vec![0; 1 << 21])), GPU::new())
    }

    #[test]
    fn memory_restrictions() {
        let mut state = mock_state();

        // Should be in OAM_SEARCH now
        state.gpu.step(&mut state.mmu);
        assert_eq!(state.gpu.MODE, GPUMode::OAM_SEARCH);

        assert_eq!(state.safe_read(VRAM_ADDR), 0xFF);
        assert_eq!(state.safe_read(VRAM_ADDR + 20), 0xFF);
        assert_eq!(state.safe_read(VRAM_ADDR + 80), 0xFF);

        assert_eq!(state.safe_read(OAM_ADDR), 0xFF);
        assert_eq!(state.safe_read(OAM_ADDR + 20), 0xFF);
        assert_eq!(state.safe_read(OAM_ADDR + 80), 0xFF);

        // Shold be in LCD_TRANSFER
        for _ in 1..OAM_SEARCH_CYCLES { state.gpu.step(&mut state.mmu) }
        assert_eq!(state.gpu.MODE, GPUMode::LCD_TRANSFER);
        
        assert_eq!(state.safe_read(VRAM_ADDR), 0xFF);
        assert_eq!(state.safe_read(VRAM_ADDR + 20), 0xFF);
        assert_eq!(state.safe_read(VRAM_ADDR + 80), 0xFF);
        
        assert_ne!(state.safe_read(OAM_ADDR), 0xFF);
        assert_ne!(state.safe_read(OAM_ADDR + 20), 0xFF);
        assert_ne!(state.safe_read(OAM_ADDR + 80), 0xFF);
    }

    #[test]
    fn vblank_interrupts() {
        let (mut mmu, mut gpu) = mock();

        // Run 30 frames
        for _ in 0..30 { 
            for _ in 0..gpu::SCANLINE_CYCLES*gpu::SCREEN_HEIGHT as u64 {
                assert!(mmu.read(ioregs::IF) & 1 == 0);
                gpu.step(&mut mmu);
            }

            // VBLANK interrupt flag should be set now
            let iflag = mmu.read(ioregs::IF);
            assert!(iflag & 1 != 0);
            mmu.write(ioregs::IF, iflag & 0xFE);

            // Finish VBLANK
            for _ in 0..gpu::SCANLINE_CYCLES*gpu::VBLANK_HEIGHT as u64 {
                assert!(mmu.read(ioregs::IF) & 1 == 0);
                gpu.step(&mut mmu);
            }
        }
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

        // Run 30 frames
        for _ in 0..30 {

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
    fn register_updates() {
        let (mut mmu, mut gpu) = mock();

        mmu.write(ioregs::LCDC, 0b10010001);
        gpu.step(&mut mmu);

        assert_eq!(gpu.LCD_DISPLAY_ENABLE, true);
        assert_eq!(gpu.WINDOW_TILE_MAP, false);
        assert_eq!(gpu.WINDOW_ENABLED, false);
        assert_eq!(gpu.TILE_ADDRESSING, true);
        assert_eq!(gpu.BG_TILE_MAP, false);
        assert_eq!(gpu.SPRITE_SIZE, false);
        assert_eq!(gpu.SPRITE_ENABLED, false);
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

    #[test]
    fn coincidence_flag() {
        let (mut mmu, mut gpu) = mock();
        mmu.write(ioregs::IF, 0);

        for i in 0..gpu::SCREEN_HEIGHT+gpu::VBLANK_HEIGHT {
            let lyc = i as u64;
            mmu.write(LYC, lyc as u8);
            gpu.LCD_DISPLAY_ENABLE = true;
            gpu.COINCIDENCE_INTERRUPT_ENABLE = true;
            gpu.MODE_0_HBLANK_INTERRUPT_ENABLE = false;
            gpu.MODE_1_VBLANK_INTERRUPT_ENABLE = false;
            gpu.MODE_2_OAM_INTERRUPT_ENABLE = false;
            gpu.flush_regs(&mut mmu);

            // All scanlnes before LYC
            for _ in 0..lyc*gpu::SCANLINE_CYCLES {
                assert_eq!(gpu.COINCIDENCE_FLAG, false);
                gpu.step(&mut mmu);
            }

            assert!((mmu.read(ioregs::IF) & 2) == 0);

            // One line of LYC
            for j in 0..gpu::SCANLINE_CYCLES {

                // Check if LYC interrupt fired
                if j == 1 {
                    println!("Line {}", lyc);
                    let iflag = mmu.read(ioregs::IF);
                    if i < gpu::SCREEN_HEIGHT { 
                        // STAT iterupt should be set if LY < 144
                        assert!((iflag & 2) != 0);
                        // Reset STAT interrupt
                        mmu.write(ioregs::IF, iflag & 0xFD); 
                    } else {
                        // LYC interrupt shouldn't be called in VBLANK
                        assert!((iflag & 2) == 0);
                    }
                }

                assert_eq!(gpu.COINCIDENCE_FLAG, true);
                gpu.step(&mut mmu);
            }

            // Rest of scanlines in current frame
            for _ in 0..gpu::SCANLINE_CYCLES*(SCREEN_HEIGHT as u64 + VBLANK_HEIGHT as u64 - lyc - 1) {
                assert_eq!(gpu.COINCIDENCE_FLAG, false);
                gpu.step(&mut mmu);
            }
        }
    }

    #[test]
    fn palette_updates() {
        let (mut mmu, mut gpu) = mock();
        
        mmu.write(ioregs::BGP, 0);
        mmu.write(ioregs::OBP_0, 0);
        mmu.write(ioregs::OBP_1, 0);
        gpu.step(&mut mmu);

        assert_eq!(gpu.BG_COLOR_3_SHADE, 0);
        assert_eq!(gpu.BG_COLOR_2_SHADE, 0);
        assert_eq!(gpu.BG_COLOR_1_SHADE, 0);
        assert_eq!(gpu.BG_COLOR_0_SHADE, 0);

        assert_eq!(gpu.OBP0_COLOR_3_SHADE, 0);
        assert_eq!(gpu.OBP0_COLOR_2_SHADE, 0);
        assert_eq!(gpu.OBP0_COLOR_1_SHADE, 0);
        assert_eq!(gpu.OBP0_COLOR_0_SHADE, 0);

        assert_eq!(gpu.OBP1_COLOR_3_SHADE, 0);
        assert_eq!(gpu.OBP1_COLOR_2_SHADE, 0);
        assert_eq!(gpu.OBP1_COLOR_1_SHADE, 0);
        assert_eq!(gpu.OBP1_COLOR_0_SHADE, 0);

        mmu.write(ioregs::BGP, 0b10111101);
        mmu.write(ioregs::OBP_0, 0b00011011);
        mmu.write(ioregs::OBP_1, 0b11001001);
        gpu.step(&mut mmu);

        assert_eq!(gpu.BG_COLOR_3_SHADE, 2);
        assert_eq!(gpu.BG_COLOR_2_SHADE, 3);
        assert_eq!(gpu.BG_COLOR_1_SHADE, 3);
        assert_eq!(gpu.BG_COLOR_0_SHADE, 1);
        assert_eq!(gpu.bg_color(3), gpu::DARK_GRAY);
        assert_eq!(gpu.bg_color(2), gpu::BLACK);
        assert_eq!(gpu.bg_color(1), gpu::BLACK);
        assert_eq!(gpu.bg_color(0), gpu::LIGHT_GRAY);

        assert_eq!(gpu.OBP0_COLOR_3_SHADE, 0);
        assert_eq!(gpu.OBP0_COLOR_2_SHADE, 1);
        assert_eq!(gpu.OBP0_COLOR_1_SHADE, 2);
        assert_eq!(gpu.OBP0_COLOR_0_SHADE, 3);
        assert_eq!(gpu.obp0_color(3), gpu::WHITE);
        assert_eq!(gpu.obp0_color(2), gpu::LIGHT_GRAY);
        assert_eq!(gpu.obp0_color(1), gpu::DARK_GRAY);
        assert_eq!(gpu.obp0_color(0), gpu::BLACK);

        assert_eq!(gpu.OBP1_COLOR_3_SHADE, 3);
        assert_eq!(gpu.OBP1_COLOR_2_SHADE, 0);
        assert_eq!(gpu.OBP1_COLOR_1_SHADE, 2);
        assert_eq!(gpu.OBP1_COLOR_0_SHADE, 1);
        assert_eq!(gpu.obp1_color(3), gpu::BLACK);
        assert_eq!(gpu.obp1_color(2), gpu::WHITE);
        assert_eq!(gpu.obp1_color(1), gpu::DARK_GRAY);
        assert_eq!(gpu.obp1_color(0), gpu::LIGHT_GRAY);
    }
}