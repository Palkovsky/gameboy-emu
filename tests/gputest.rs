extern crate gameboy;

#[cfg(test)]
mod gputest {
    use gameboy::*;

    fn gen() -> (MMU<mbc::MBC1>, GPU) {
        let mut mmu = mem::MMU::new(mbc::MBC1::new(vec![0; 1 << 21]));
        let gpu = GPU::new(&mut mmu);
        (mmu, gpu)
    }

    fn gen_state() -> State<mbc::MBC1> {
        State::new(mbc::MBC1::new(vec![0; 1 << 21]))
    }

    #[test]
    fn memory_restrictions() {
        let mut state = gen_state();

        // Should be in OAM_SEARCH now
        assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::OAM_SEARCH);

        assert_eq!(state.safe_read(VRAM_ADDR), 0xFF);
        assert_eq!(state.safe_read(VRAM_ADDR + 20), 0xFF);
        assert_eq!(state.safe_read(VRAM_ADDR + 80), 0xFF);

        assert_eq!(state.safe_read(OAM_ADDR), 0xFF);
        assert_eq!(state.safe_read(OAM_ADDR + 20), 0xFF);
        assert_eq!(state.safe_read(OAM_ADDR + 80), 0xFF);

        // Shold be in LCD_TRANSFER
        state.gpu.step(&mut state.mmu);
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
        let (mut mmu, mut gpu) = gen();

        // VBLANK INT shoul be reset
        assert!(mmu.read(ioregs::IF) & 1 == 0);

        // 10 frames
        for _ in 0..10 { 
            // Should be on start of scanline
            assert_eq!(GPU::MODE(&mut mmu), GPUMode::OAM_SEARCH);

            // Screen render
            for _ in 0..gpu::SCANLINE_STEPS*gpu::SCREEN_HEIGHT as u64 {
                assert!(mmu.read(ioregs::IF) & 1 == 0);
                gpu.step(&mut mmu);
            }

            // Should be in VBLANK
            assert_eq!(GPU::MODE(&mut mmu), GPUMode::VBLANK);

            // VBLANK interrupt flag should be set now
            let iflag = mmu.read(ioregs::IF);
            assert!(iflag & 1 != 0);
            mmu.write(ioregs::IF, iflag & 0xFE);

            // Finish VBLANK
            gpu.step(&mut mmu);
        }
    }

    #[test]
    fn ly_updates() {
        let (mut mmu, mut gpu) = gen();

        // 10 frames
        for _ in 0..10 {
            assert_eq!(GPU::MODE(&mut mmu), GPUMode::OAM_SEARCH);

            for ly in 0..gpu::SCREEN_HEIGHT {
                assert_eq!(mmu.read(ioregs::LY), ly as u8);
                assert_eq!(GPU::LY(&mut mmu), ly as u8);
                for _ in 0..gpu::SCANLINE_STEPS { gpu.step(&mut mmu); }
            }

            assert_eq!(GPU::MODE(&mut mmu), GPUMode::VBLANK);
            gpu.step(&mut mmu);
        }
    }

    #[test]
    fn mode_changes() {
        let (mut mmu, mut gpu) = gen();

        // 10 frames
        for _ in 0..10 {            
            for _ in 0..gpu::SCREEN_HEIGHT {
                // Scanline starts with OAM_SEARCH
                assert_eq!(GPU::MODE(&mut mmu), GPUMode::OAM_SEARCH);

                // Then there is LCD_TRANSFER
                gpu.step(&mut mmu);
                assert_eq!(GPU::MODE(&mut mmu), gpu::GPUMode::LCD_TRANSFER);

                // Then HBLANK
                gpu.step(&mut mmu);
                assert_eq!(GPU::MODE(&mut mmu), gpu::GPUMode::HBLANK);

                // Back to OAM
                gpu.step(&mut mmu);
            }

            // VBLANK at the end
            assert_eq!(GPU::MODE(&mut mmu), gpu::GPUMode::VBLANK);
            gpu.step(&mut mmu);
        }
    }

    #[test]
    fn register_updates() {
        let (mut mmu, mut gpu) = gen();

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
        let mut state = gen_state();

        // STAT interrupt shouldn't be set        
        assert!((state.mmu.read(ioregs::IF) & 2) == 0);

        // Configure GPU
        GPU::_LCD_DISPLAY_ENABLE(&mut state.mmu, true);
        GPU::_COINCIDENCE_INTERRUPT_ENABLE(&mut state.mmu, true);
        GPU::_MODE_0_HBLANK_INTERRUPT_ENABLE(&mut state.mmu, false);
        GPU::_MODE_1_VBLANK_INTERRUPT_ENABLE(&mut state.mmu, false);
        GPU::_MODE_2_OAM_INTERRUPT_ENABLE(&mut state.mmu, false);

        for i in 0..gpu::SCREEN_HEIGHT {
            let lyc = i as u64;
            state.safe_write(LYC, lyc as u8);
    
            // All scanlnes before LYC
            let updates = if lyc == 0 { 0 } else { lyc*gpu::SCANLINE_STEPS - 1};
            for _ in 0..updates {
                state.gpu.step(&mut state.mmu);
                assert_eq!(GPU::COINCIDENCE_FLAG(&mut state.mmu), false);
            }

            if lyc != 0 {
                // HBLANK of line before LYC
                assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::HBLANK);
                // Flag should be set
                assert_eq!(GPU::COINCIDENCE_FLAG(&mut state.mmu), false);
                // But interrupt shouldn't since it triggers DURING OAM Search
                assert!((state.mmu.read(ioregs::IF) & 2) == 0);
                // Finish HBLANK of line before
                state.gpu.step(&mut state.mmu);
            }

            assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::OAM_SEARCH);
            // Flag should be set
            assert_eq!(GPU::COINCIDENCE_FLAG(&mut state.mmu), true);
            // But interrupt shouldn't since it triggers DURING OAM Search
            assert!((state.mmu.read(ioregs::IF) & 2) == 0);

            // Finish OAM search
            state.gpu.step(&mut state.mmu);
            assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::LCD_TRANSFER);
            // Flag still should be set
            assert_eq!(GPU::COINCIDENCE_FLAG(&mut state.mmu), true);            
            // STAT interrupt flag should be set now
            let iflag = state.mmu.read(ioregs::IF);
            assert!((iflag & 2) != 0);
            state.safe_write(ioregs::IF, iflag & 0xFD); 
            
            // Finish LCD transfer
            state.gpu.step(&mut state.mmu);
            assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::HBLANK);
            assert!((state.mmu.read(ioregs::IF) & 2) == 0); // Shouln't set interrupt for same line
            assert_eq!(GPU::COINCIDENCE_FLAG(&mut state.mmu), true);

            // Finish HBLANK
            state.gpu.step(&mut state.mmu);
            if GPU::LY(&mut state.mmu) == gpu::SCREEN_HEIGHT as u8 {
                assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::VBLANK);
            } else {
                assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::OAM_SEARCH);
            }
            assert!((state.mmu.read(ioregs::IF) & 2) == 0); // Shouln't set interrupt for same line
            assert_eq!(GPU::COINCIDENCE_FLAG(&mut state.mmu), false);

            // Rest of steps in current
            for _ in 0..gpu::SCANLINE_STEPS*(SCREEN_HEIGHT as u64 - lyc - 1) + 1{
                // println!("TEST | LYC {}, LINE {}", lyc, j);
                assert_eq!(GPU::COINCIDENCE_FLAG(&mut state.mmu), false);
                state.gpu.step(&mut state.mmu);
            }

            assert_eq!(GPU::MODE(&mut state.mmu), GPUMode::OAM_SEARCH);
        }
    }

    #[test]
    fn palette_updates() {
        let (mut mmu, mut gpu) = gen();
        
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