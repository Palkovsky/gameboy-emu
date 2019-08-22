extern crate gameboy;

#[cfg(test)]
mod gputest {
    use gameboy::*;

    fn mock() -> (MMU<mbc::MBC1>, GPU) {
        let mut mmu = mem::MMU::new(mbc::MBC1::new(vec![0; 1 << 21]));
        let gpu = GPU::new();
        GPU::_MODE(&mut mmu, GPUMode::OAM_SEARCH);

        let lyc = mmu.read(ioregs::LYC);
        let ly = mmu.read(ioregs::LY);
        GPU::_COINCIDENCE_FLAG(&mut mmu, lyc == ly);

        (mmu, gpu)
    }

    fn mock_state() -> State<mbc::MBC1> {
        State::new(mbc::MBC1::new(vec![0; 1 << 21]))
    }

    #[test]
    fn memory_restrictions() {
        let mut state = mock_state();

        // Should be in OAM_SEARCH now
        state.gpu.step(&mut state.mmu);
        assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::OAM_SEARCH);

        assert_eq!(state.safe_read(VRAM_ADDR), 0xFF);
        assert_eq!(state.safe_read(VRAM_ADDR + 20), 0xFF);
        assert_eq!(state.safe_read(VRAM_ADDR + 80), 0xFF);

        assert_eq!(state.safe_read(OAM_ADDR), 0xFF);
        assert_eq!(state.safe_read(OAM_ADDR + 20), 0xFF);
        assert_eq!(state.safe_read(OAM_ADDR + 80), 0xFF);

        // Shold be in LCD_TRANSFER
        for _ in 1..OAM_SEARCH_CYCLES { state.gpu.step(&mut state.mmu) }
        assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::LCD_TRANSFER);
        
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

        // 10 frames
        for _ in 0..10 { 
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

        // 10 frames
        for _ in 0..10 {
            for ly in 0..(gpu::SCREEN_HEIGHT + gpu::VBLANK_HEIGHT) {
                assert_eq!(mmu.read(ioregs::LY), ly as u8);
                for _ in 0..gpu::SCANLINE_CYCLES { gpu.step(&mut mmu); }
            }
        }
    }

    #[test]
    fn mode_changes() {
        let (mut mmu, mut gpu) = mock();

        // 10 frames
        for _ in 0..10 {
            // Check if OAM/LCD/HBLANK states take proper number of cycles
            for _ in 0..gpu::SCREEN_HEIGHT {
                for _ in 0..gpu::OAM_SEARCH_CYCLES {
                    assert_eq!(GPU::MODE(&mut mmu), gpu::GPUMode::OAM_SEARCH);
                    gpu.step(&mut mmu);
                }

                for _ in 0..gpu::LCD_TRANSFER_CYCLES {
                    assert_eq!(GPU::MODE(&mut mmu), gpu::GPUMode::LCD_TRANSFER);
                    gpu.step(&mut mmu);
                }

                for _ in 0..gpu::HBLANK_CYCLES {
                    assert_eq!(GPU::MODE(&mut mmu), gpu::GPUMode::HBLANK);
                    gpu.step(&mut mmu);
                }
            }

            // Check if VBLANK takes proper number of cycles
            for _ in 0..gpu::VBLANK_CYCLES {
                assert_eq!(GPU::MODE(&mut mmu), gpu::GPUMode::VBLANK);
                gpu.step(&mut mmu);
            }
        }
    }

    #[test]
    fn register_updates() {
        let (mut mmu, mut gpu) = mock();

        mmu.write(ioregs::LCDC, 0b10010001);
        gpu.step(&mut mmu);

        assert_eq!(GPU::LCD_DISPLAY_ENABLE(&mut mmu), true);
        assert_eq!(GPU::WINDOW_TILE_MAP(&mut mmu), false);
        assert_eq!(GPU::WINDOW_ENABLED(&mut mmu), false);
        assert_eq!(GPU::TILE_ADDRESSING(&mut mmu), true);
        assert_eq!(GPU::BG_TILE_MAP(&mut mmu), false);
        assert_eq!(GPU::SPRITE_SIZE(&mut mmu), false);
        assert_eq!(GPU::SPRITE_ENABLED(&mut mmu), false);
        assert_eq!(GPU::DISPLAY_PRIORITY(&mut mmu), true);

        mmu.write(ioregs::STAT, 0b10010000);
        gpu.step(&mut mmu);

        assert_eq!(GPU::COINCIDENCE_INTERRUPT_ENABLE(&mut mmu), false);
        assert_eq!(GPU::MODE_2_OAM_INTERRUPT_ENABLE(&mut mmu), false);
        assert_eq!(GPU::MODE_1_VBLANK_INTERRUPT_ENABLE(&mut mmu), true);
        assert_eq!(GPU::MODE_0_HBLANK_INTERRUPT_ENABLE(&mut mmu), false);
        //assert_eq!(GPU::COINCIDENCE_FLAG(&mut mmu), false);
        //assert_eq!(GPU::MODE(&mut mmu), gpu::GPUMode::OAM_SEARCH);
    }

    #[test]
    fn coincidence_flag() {
        let (mut mmu, mut gpu) = mock();
        mmu.write(ioregs::IF, 0);

        for i in 1..gpu::SCREEN_HEIGHT {
            let lyc = i as u64;
            mmu.write(LYC, lyc as u8);
            GPU::_LCD_DISPLAY_ENABLE(&mut mmu, true);
            GPU::_COINCIDENCE_INTERRUPT_ENABLE(&mut mmu, true);
            GPU::_MODE_0_HBLANK_INTERRUPT_ENABLE(&mut mmu, false);
            GPU::_MODE_1_VBLANK_INTERRUPT_ENABLE(&mut mmu, false);
            GPU::_MODE_2_OAM_INTERRUPT_ENABLE(&mut mmu, false);

            // All scanlnes before LYC
            for _ in 0..lyc*gpu::SCANLINE_CYCLES-1 {
                gpu.step(&mut mmu);
                assert_eq!(GPU::COINCIDENCE_FLAG(&mut mmu), false);
            }

            assert!((mmu.read(ioregs::IF) & 2) == 0);
            gpu.step(&mut mmu);

            // One line of LYC
            for j in 0..gpu::SCANLINE_CYCLES {

                // Check if LYC interrupt fired
                if j == 1 {
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

                assert_eq!(GPU::COINCIDENCE_FLAG(&mut mmu), true);
                gpu.step(&mut mmu);

            }

            // Rest of scanlines in current frame
            for _ in 0..gpu::SCANLINE_CYCLES*(SCREEN_HEIGHT as u64 + VBLANK_HEIGHT as u64 - lyc - 1) {
                // println!("TEST | LYC {}, LINE {}", lyc, j);
                assert_eq!(GPU::COINCIDENCE_FLAG(&mut mmu), false);
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

        assert_eq!(GPU::BG_COLOR_3_SHADE(&mut mmu), 0);
        assert_eq!(GPU::BG_COLOR_2_SHADE(&mut mmu), 0);
        assert_eq!(GPU::BG_COLOR_1_SHADE(&mut mmu), 0);
        assert_eq!(GPU::BG_COLOR_0_SHADE(&mut mmu), 0);

        assert_eq!(GPU::OBP0_COLOR_3_SHADE(&mut mmu), 0);
        assert_eq!(GPU::OBP0_COLOR_2_SHADE(&mut mmu), 0);
        assert_eq!(GPU::OBP0_COLOR_1_SHADE(&mut mmu), 0);

        assert_eq!(GPU::OBP1_COLOR_3_SHADE(&mut mmu), 0);
        assert_eq!(GPU::OBP1_COLOR_2_SHADE(&mut mmu), 0);
        assert_eq!(GPU::OBP1_COLOR_1_SHADE(&mut mmu), 0);

        mmu.write(ioregs::BGP, 0b10111101);
        mmu.write(ioregs::OBP_0, 0b00011011);
        mmu.write(ioregs::OBP_1, 0b11001001);
        gpu.step(&mut mmu);

        assert_eq!(GPU::BG_COLOR_3_SHADE(&mut mmu), 2);
        assert_eq!(GPU::BG_COLOR_2_SHADE(&mut mmu), 3);
        assert_eq!(GPU::BG_COLOR_1_SHADE(&mut mmu), 3);
        assert_eq!(GPU::BG_COLOR_0_SHADE(&mut mmu), 1);
        assert_eq!(GPU::bg_color(&mut mmu, 3), gpu::DARK_GRAY);
        assert_eq!(GPU::bg_color(&mut mmu, 2), gpu::BLACK);
        assert_eq!(GPU::bg_color(&mut mmu, 1), gpu::BLACK);
        assert_eq!(GPU::bg_color(&mut mmu, 0), gpu::LIGHT_GRAY);

        assert_eq!(GPU::OBP0_COLOR_3_SHADE(&mut mmu), 0);
        assert_eq!(GPU::OBP0_COLOR_2_SHADE(&mut mmu), 1);
        assert_eq!(GPU::OBP0_COLOR_1_SHADE(&mut mmu), 2);
        assert_eq!(GPU::obp0_color(&mut mmu, 3), gpu::WHITE);
        assert_eq!(GPU::obp0_color(&mut mmu, 2), gpu::LIGHT_GRAY);
        assert_eq!(GPU::obp0_color(&mut mmu, 1), gpu::DARK_GRAY);
        assert_eq!(GPU::obp0_color(&mut mmu, 0), gpu::TRANSPARENT);

        assert_eq!(GPU::OBP1_COLOR_3_SHADE(&mut mmu), 3);
        assert_eq!(GPU::OBP1_COLOR_2_SHADE(&mut mmu), 0);
        assert_eq!(GPU::OBP1_COLOR_1_SHADE(&mut mmu), 2);
        assert_eq!(GPU::obp1_color(&mut mmu, 3), gpu::BLACK);
        assert_eq!(GPU::obp1_color(&mut mmu, 2), gpu::WHITE);
        assert_eq!(GPU::obp1_color(&mut mmu, 1), gpu::DARK_GRAY);
        assert_eq!(GPU::obp1_color(&mut mmu, 0), gpu::TRANSPARENT);
    }
}