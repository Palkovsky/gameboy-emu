extern crate gameboy;

#[cfg(test)]
mod cputest {
    use gameboy::*;

    const NOP: u8 = 0x00;

    fn gen() -> Runtime<mbc::MBC3> { 
        Runtime::new(mbc::MBC3::new(vec![0; 1 << 21]))
    }

    fn gen_with_code(code: Vec<u8>) -> Runtime<mbc::MBC3> {
        let mut bytes = vec![0; 1 << 21];
        for (i, b) in code.into_iter().enumerate() { bytes[i] = b; }
        let mut res = Runtime::new(mbc::MBC3::new(bytes));
        
        res.state.mmu.booting(false); // We're testing instructions so bootrom should be disabled
        res.cpu.STOP = false;
        res.cpu.HALT = false;

        res
    }

    #[test]
    fn cb_instructions() {
        let mut runtime = gen_with_code(vec![
            0x3E, 0x00, // LD A, 0x00
            0xCB, 0xD7, // SET 2, A
            0xCB, 0x57, // BIT 2, A
        ]);

        runtime.step();
        assert_eq!(runtime.cpu.A, 0x00);

        runtime.step();
        assert_eq!(runtime.cpu.A, 1 << 2);
        runtime.cpu.Z = false;

        runtime.step();
        assert_eq!(runtime.cpu.Z, true);
    }

    #[test]
    fn simple_loop() {
        let mut runtime = gen_with_code(vec![
            0x3E, 0x00, // LD A, 0x00
            0x3C, // INC A
            0xC3, 0x02, 0x00, // JP 0x0002
        ]);

        runtime.step();
        assert_eq!(runtime.cpu.PC.val(), 0x0002);
        assert_eq!(runtime.cpu.A, 0x00);
        
        for i in 1..50 {
            runtime.step();
            assert_eq!(runtime.cpu.PC.val(), 0x0003);
            assert_eq!(runtime.cpu.A, i);

            runtime.step();
            assert_eq!(runtime.cpu.PC.val(), 0x0002);
            assert_eq!(runtime.cpu.A, i);
        }
    }

    #[test]
    fn rotations() {
        let mut runtime = gen_with_code(vec![
            0x3E, 0b10100011, // LD A,0b10100011
            0x07, // RLCA
            0x17, // RLA
            0x0F, // RRCA
            0x1F, // RRA
        ]);
        runtime.cpu.C = false;

        runtime.step();
        assert_eq!(runtime.cpu.A, 0b10100011);

        runtime.step();
        assert_eq!(runtime.cpu.A, 0b01000111);
        assert_eq!(runtime.cpu.C, true);

        runtime.step();
        assert_eq!(runtime.cpu.A, 0b10001111);
        assert_eq!(runtime.cpu.C, false);
        
        runtime.step();
        assert_eq!(runtime.cpu.A, 0b11000111);
        assert_eq!(runtime.cpu.C, true);

        runtime.step();
        assert_eq!(runtime.cpu.A, 0b11100011);
        assert_eq!(runtime.cpu.C, true);
    }

    #[test]
    fn stack_push_pop() {
        let mut runtime = gen_with_code(vec![
            0xD5, // PUSH DE
            0xF1, // POP AF
        ]);

        runtime.cpu.Z = false;
        runtime.cpu.N = true;
        runtime.cpu.H = false;
        runtime.cpu.C = true;

        runtime.cpu.DE.set_up(0x55);
        runtime.cpu.DE.set_low(0b10100000);
        let sp = runtime.cpu.SP;

        runtime.step();
        assert_eq!(runtime.state.safe_read(runtime.cpu.SP), 0b10100000);
        assert_eq!(runtime.state.safe_read(runtime.cpu.SP + 1), 0x55);
        assert_eq!(runtime.cpu.SP, sp - 2);

        runtime.step();
        assert_eq!(runtime.cpu.A, 0x55);
        assert_eq!(runtime.cpu.Z, true);
        assert_eq!(runtime.cpu.N, false);
        assert_eq!(runtime.cpu.H, true);
        assert_eq!(runtime.cpu.C, false);
        assert_eq!(runtime.cpu.SP, sp);
    }

    #[test]
    fn pre_increment_post_decrement() {
        let mut runtime = gen_with_code(vec![
            0x21, 0x00, 0xC0, // LD HL, $C000
            0x3E, 0x69, // LD A, $69
            0x22, // LD (HL+), A
            0x3E, 0x70, // LD A, $70
            0x3A, // LD A, (HL-)
            0x3A, 
        ]);

        assert_ne!(runtime.cpu.HL.val(), 0xC000);

        runtime.step();
        assert_eq!(runtime.cpu.HL.val(), 0xC000);
        assert_ne!(runtime.cpu.A, 0x69);
        
        runtime.step();
        assert_eq!(runtime.cpu.A, 0x69);
        let hl = runtime.cpu.HL.val();
        assert_eq!(runtime.state.safe_read(hl), 0x00);

        runtime.step();
        assert_eq!(runtime.state.safe_read(0xC000), 0x69);
        assert_eq!(runtime.cpu.HL.val(), hl + 1);
        assert_eq!(runtime.cpu.HL.val(), 0xC001);

        runtime.step();
        assert_eq!(runtime.cpu.A, 0x70);
        let hl = runtime.cpu.HL.val();

        runtime.step();
        assert_eq!(runtime.cpu.A, 0x69);
        assert_eq!(runtime.cpu.HL.val(), hl - 1);
        assert_eq!(runtime.cpu.HL.val(), 0xC000);
    }

    #[test]
    fn zero_page_moves(){
        let mut runtime = gen_with_code(vec![
            0x0E, 0x13, // LD C, $13
            0xF2, // LD A, (C)
        ]);

        // We assume register values are different from what we expect after modification.
        assert_ne!(runtime.cpu.BC.low(), 0x00);
        assert_ne!(runtime.cpu.A, 0x21);
        assert_eq!(runtime.cpu.PC.val(), 0x0000);

        {
            let mmu = &mut runtime.state.mmu;
            mmu.write(ZP_ADDR + 0x13, 0x21);
        }

        runtime.step();
        assert_eq!(runtime.cpu.BC.low(), 0x13);
        assert_ne!(runtime.cpu.A, 0x00);
        assert_eq!(runtime.cpu.PC.val(), 0x0002);

        runtime.step();
        assert_eq!(runtime.cpu.A, 0x21);
        assert_eq!(runtime.cpu.PC.val(), 0x0003);

    }

    // This test case tests how CPU behaves with defult initialization(HALT=false, STOP=false).
    // I test how PC changes when executing NOPs.
    #[test]
    fn nop_updates() {
        let mut runtime = gen();
        
        assert_eq!(runtime.cpu.PC.val(), 0x0000); 

        // Make sure there are NOPs only
        runtime.state.mmu.booting(false);
        for off in 0..256 { assert_eq!(runtime.state.mmu.read(0x0000 + off), NOP); }

        // Check if PC incremented correctly
        for off in 0..256 {
            runtime.step();
            assert_eq!(runtime.cpu.PC.val(), 0x0001 + off); 
        }
    }

    #[test]
    fn halt_flag() {
        let mut runtime = gen();
        
        runtime.cpu.HALT = true;
        assert_eq!(runtime.cpu.PC.val(), 0x0000);

        // Try updating and make sure PC won't move forward.
        for _ in 0..256 {
            runtime.step(); 
            assert_eq!(runtime.cpu.PC.val(), 0x0000);
        }

        // Try unhalting by sending an interrupt
        // Enable STAT interrupt
        runtime.state.safe_write(ioregs::IE, 2);
        // Request STAT interrupt
        runtime.state.safe_write(ioregs::IF, 2);

        assert_eq!(runtime.cpu.HALT, true);
        assert_eq!(runtime.cpu.IME, true);
        runtime.step(); // Perform next instruction cycle
        assert_eq!(runtime.cpu.HALT, false);
        assert_eq!(runtime.cpu.IME, false);
        assert_eq!(runtime.cpu.PC.val(), 0x0048);
    }
}