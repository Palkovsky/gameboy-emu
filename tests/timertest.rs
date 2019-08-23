extern crate gameboy;
extern crate rand;

#[cfg(test)]
mod timertest {
    use gameboy::*;
    use rand::Rng;

    fn mock_state() -> State<mbc::MBC1> {
        State::new(mbc::MBC1::new(vec![0; 1 << 21]))
    }

    #[test]
    fn div_counter() {
        let mut state = mock_state();
        let mmu = &mut state.mmu;
        let timer = &mut state.timer; 

        for i in 0..700 {
            let tick = (i % 256) as u8; 
            for _ in 0..timer::STEPS_16384HZ {
                assert_eq!(Timer::DIV(mmu), tick);
                timer.step(mmu);
            }
        }

        assert_ne!(Timer::DIV(mmu), 0x00);
        assert_ne!(state.safe_read(ioregs::DIV), 0x00);

        state.safe_write(ioregs::DIV, 0x69);
        let mmu = &mut state.mmu;
        
        assert_eq!(Timer::DIV(mmu), 0);
        assert_eq!(state.safe_read(ioregs::DIV), 0x00);
    }

    #[test]
    fn tima_counter() {
        let mut state = mock_state();
        let mut rng = rand::thread_rng();

        let steps = [timer::STEPS_4096HZ, timer::STEPS_16384HZ, timer::STEPS_65536HZ, timer::STEPS_262144HZ];
        let modes = [TimerMode::FQ_4096HZ, TimerMode::FQ_16384HZ, TimerMode::FQ_65536HZ, TimerMode::FQ_262144HZ];
        let masks = [0, 3, 2, 1];

        for ((steps, mode), mask) in steps.into_iter().zip(modes.into_iter()).zip(masks.into_iter()) {
            state.safe_write(ioregs::TAC, *mask);
            assert_eq!(Timer::STOPPED(&mut state.mmu), false);
            assert_eq!(Timer::MODE(&mut state.mmu), *mode);

            state.safe_write(ioregs::TIMA, 0);
            assert_eq!(Timer::TIMA(&mut state.mmu), 0);

            let tma: u8 = rng.gen();
            let mut count = 0u8;
            state.safe_write(ioregs::TMA, tma);
            assert_eq!(Timer::TMA(&mut state.mmu), tma);

            for _ in 0..700 {
                let overflow = Timer::TIMA(&mut state.mmu) == 0xFF;
                for _ in 0..*steps {
                    assert_eq!(Timer::TIMA(&mut state.mmu), count);
                    state.timer.step(&mut state.mmu);
                }
                count = if overflow { tma } else { count+1 };
            }
        } 
    }

    // This test case covers updates with TIMER_STOPED flag set.
    // Expecting DIV to count anyway and TIMA to be stopped.
    #[test]
    fn timer_disabled() {
        let mut state = mock_state();

        state.safe_write(ioregs::TAC, 0b111);
        assert_eq!(Timer::STOPPED(&mut state.mmu), true);
        assert_eq!(Timer::MODE(&mut state.mmu), TimerMode::FQ_16384HZ);
        
        assert_eq!(Timer::DIV(&mut state.mmu), 0);
        assert_eq!(state.safe_read(ioregs::DIV), 0);
        assert_eq!(Timer::TIMA(&mut state.mmu), 0);
        assert_eq!(state.safe_read(ioregs::TIMA), 0);

        for _ in 0..timer::STEPS_16384HZ*3 { state.timer.step(&mut state.mmu); }

        assert_eq!(Timer::TIMA(&mut state.mmu), 0);
        assert_eq!(state.safe_read(ioregs::TIMA), 0);
        assert_ne!(Timer::DIV(&mut state.mmu), 0);
        assert_ne!(state.safe_read(ioregs::DIV), 0);
    }

    // This test covers case when write to DIV/TIMA would leave internal clock unchanged.
    // Expected behavior for internal clocks would be to set values with defaults.
    #[test]
    fn tima_runtime_updates() {
        let mut state = mock_state();

        state.safe_write(ioregs::TAC, 0b000);
        assert_eq!(Timer::STOPPED(&mut state.mmu), false);
        assert_eq!(Timer::MODE(&mut state.mmu), TimerMode::FQ_4096HZ);

        // Loop for full clock and some extra few machine cycles
        for _ in 0..timer::STEPS_4096HZ + 200 { state.timer.step(&mut state.mmu); }
        assert_eq!(Timer::TIMA(&mut state.mmu), 1);
        assert_eq!(state.safe_read(ioregs::TIMA), 1);

        // Now set to some arbitrary value
        state.safe_write(ioregs::TIMA, 21);
        assert_eq!(Timer::TIMA(&mut state.mmu), 21);
        assert_eq!(state.safe_read(ioregs::TIMA), 21);

        // IMPORTATNT: If internal clocks wouldn't be reset the time value would go to 23
        for _ in 0..timer::STEPS_4096HZ { state.timer.step(&mut state.mmu); }

        assert_eq!(Timer::TIMA(&mut state.mmu), 22);
        assert_eq!(state.safe_read(ioregs::TIMA), 22);
   }

    #[test]
    fn div_runtime_updates() {
        let mut state = mock_state();

        assert_eq!(Timer::DIV(&mut state.mmu), 0);

        // Loop for full clock and some extra few machine cycles
        for _ in 0..timer::STEPS_16384HZ*4 + 50 { state.timer.step(&mut state.mmu); }
        assert_eq!(Timer::DIV(&mut state.mmu), 4);
        assert_eq!(state.safe_read(ioregs::DIV), 4);

        // Now set to some arbitrary value
        state.safe_write(ioregs::DIV, 21);
        assert_eq!(Timer::DIV(&mut state.mmu), 0);
        assert_eq!(state.safe_read(ioregs::DIV), 0);

        // IMPORTATNT: If internal clocks wouldn't be reset the time value would go to 23
        for _ in 0..timer::STEPS_16384HZ { state.timer.step(&mut state.mmu); }

        assert_eq!(Timer::TIMA(&mut state.mmu), 1);
        assert_eq!(state.safe_read(ioregs::TIMA), 1);
   }
}