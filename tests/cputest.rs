extern crate gameboy;


#[cfg(test)]
mod cputest {
    use gameboy::*;

    const NOP: u8 = 0x00;

    fn gen() -> Runtime<mbc::MBC3> { Runtime::new(mbc::MBC3::new(vec![0; 1 << 21])) }

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