#![allow(non_snake_case, non_camel_case_types)]

use super::*;

const CPU_FREQUENCY: u32 = 1 << 20;
const SEQUENCER_FREQUENCY: u32 = 512;
const SEQUENCER_UPDATE_RATE: u16 = (CPU_FREQUENCY/SEQUENCER_FREQUENCY) as u16;
const SEQUENCER_STEP_COUNT: u16 = 8;
const DUTY_CYCLE_COUNT: u16 = 4;
const DUTY_CYCLE_STEPS: u16 = 8;
pub const BUFF_SIZE: usize = 1024;
pub const PLAYBACK_FREQUENCY: u32 = 44100;
const SAMPLE_APPEND_RATE: u16 = (CPU_FREQUENCY/PLAYBACK_FREQUENCY) as u16 + 1;
const WAVE_RAM_SAMPLE_COUNT: usize = 32;
const WAVE_RAM_BASE: u16 = 0xFF30;
const NOISE_LSFR_SIZE: usize = 16;

const DUTY_CYCLES: [[bool; DUTY_CYCLE_STEPS as usize]; DUTY_CYCLE_COUNT as usize] = [ 
    [false, true, true, true, true, true, true, true], // 12.5%
    [false, false, true, true, true, true, true, true], // 25%
    [false, false, false, false, true, true, true, true], // 50%
    [false, false, false, false, false, false, true, true], // 75%
];

trait SquareWaveRegisters {
    fn SWEEP_TIME(&self, mmu: &mut MMU<impl BankController>) -> u16;
    fn SWEEP_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8;
    fn SWEEP_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn SOUND_LENGTH(&self, mmu: &mut MMU<impl BankController>) -> u16;
    fn WAVE_DUTY(&self, mmu: &mut MMU<impl BankController>) -> u8;
    fn ENVELOPE_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8;
    fn ENVELOPE_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn INITIAL_VOLUME(&self, mmu: &mut MMU<impl BankController>) -> u16;
    fn FREQ(&self, mmu: &mut MMU<impl BankController>) -> u16;
    fn COUNTER_CONSECUTIVE_SELECT(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn INITIAL(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn _INITIAL(&self, mmu: &mut MMU<impl BankController>, value: bool);
    fn ENABLED(&self, mmu: &mut MMU<impl BankController>) -> bool;
    fn _ENABLED(&self, mmu: &mut MMU<impl BankController>, value: bool);
}

struct Channel1Regs;
impl SquareWaveRegisters for Channel1Regs {
    // NR 10 - Sweep register
    fn SWEEP_TIME(&self, mmu: &mut MMU<impl BankController>) -> u16         { (mmu.read(ioregs::NR_10) >> 4) as u16}
    fn SWEEP_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8        { mmu.read(ioregs::NR_10) & 7 }
    fn SWEEP_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool   { mmu.read_bit(ioregs::NR_10, 3) }

    // NR 11 - Length and wave duty registers
    fn SOUND_LENGTH(&self, mmu: &mut MMU<impl BankController>) -> u16 { (mmu.read(ioregs::NR_11) & 0x3F) as u16 }
    fn WAVE_DUTY(&self, mmu: &mut MMU<impl BankController>) -> u8    { mmu.read(ioregs::NR_11) >> 6 }

    // NR 12 - Volume Envelope register
    fn ENVELOPE_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8       { mmu.read(ioregs::NR_12) & 7 }
    fn ENVELOPE_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool  { mmu.read_bit(ioregs::NR_22, 3) }
    fn INITIAL_VOLUME(&self, mmu: &mut MMU<impl BankController>) -> u16       { (mmu.read(ioregs::NR_12) >> 4)  as u16 }

    // NR13 and NR14 - frequency
    fn FREQ(&self, mmu: &mut MMU<impl BankController>) -> u16 {
        (((mmu.read(ioregs::NR_14) & 7) as u16) << 8) + mmu.read(ioregs::NR_13) as u16
    }
    // NR 14 - Counter/Consecutive selection and initial flags
    fn COUNTER_CONSECUTIVE_SELECT(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_14, 6) }
    fn INITIAL(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_14, 7) }
    fn _INITIAL(&self, mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_14, 7, value) }

    // NR52 - Sound ON/OFF
    fn ENABLED(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_52, 0) }
    fn _ENABLED(&self, mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_52, 0, value) }
}

struct Channel2Regs;
impl SquareWaveRegisters for Channel2Regs {
    // No sweep in channel2
    fn SWEEP_TIME(&self, mmu: &mut MMU<impl BankController>) -> u16 { 0 }
    fn SWEEP_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8 { 0 }
    fn SWEEP_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool { false }

    // NR 21 - Length and wave duty registers
    fn SOUND_LENGTH(&self, mmu: &mut MMU<impl BankController>) -> u16 { (mmu.read(ioregs::NR_21) & 0x3F) as u16 }
    fn WAVE_DUTY(&self, mmu: &mut MMU<impl BankController>) -> u8 { mmu.read(ioregs::NR_21) >> 6 }

    // NR 22 - Volume Envelope register
    fn ENVELOPE_SHIFTS(&self, mmu: &mut MMU<impl BankController>) -> u8 { mmu.read(ioregs::NR_22) & 7 }
    fn ENVELOPE_DIRECTION(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_22, 3) }
    fn INITIAL_VOLUME(&self, mmu: &mut MMU<impl BankController>) -> u16 { (mmu.read(ioregs::NR_22) >> 4) as u16 }

    // NR23 and NR24 - frequency
    fn FREQ(&self, mmu: &mut MMU<impl BankController>) -> u16 {
        (((mmu.read(ioregs::NR_24) & 7) as u16) << 8) + mmu.read(ioregs::NR_23) as u16
    }
    // NR 24 - Counter/Consecutive selection and initial flags
    fn COUNTER_CONSECUTIVE_SELECT(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_24, 6)  }
    fn INITIAL(&self, mmu: &mut MMU<impl BankController>) -> bool {mmu.read_bit(ioregs::NR_24, 7) }
    fn _INITIAL(&self, mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_24, 7, value) }

    // NR52 - Sound ON/OFF
    fn ENABLED(&self, mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_52, 1) }
    fn _ENABLED(&self, mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_52, 1, value) }
}

struct SquareWaveChannel<T: SquareWaveRegisters> {
    /* frequency with sweep function transforms */
    frequency: u16,
    /* volume with envelope function transforms */
    volume: u16,
    /* Decremented by frame sequencer. 256Hz */
    length: u16,
    /* Initialized with (2048-frequency). Decremented in each CPU cycle. If 0 reached, increment duty cycle. */
    timer: u16,
    /* 8 duty cycles. Wraps when over 7. */
    duty_cycle: u16,
    /* sweep timer */
    sweep_timer: u16,
    envelope_count: u8,
    /* Output buffer */
    buff: Vec<i16>,
    /* Used to fillup buffer for player with PLAYBACK_FREQUENCY sampling rate, not CPU_FREQUENCY */
    sample_counter: u16,
    /* Provides access to memory mapped registers */
    regs: T,
}

impl <T: SquareWaveRegisters>SquareWaveChannel<T> {
    fn new(mmu: &mut MMU<impl BankController>, regs: T) -> Self { 
        Self {
            frequency:      regs.FREQ(mmu),
            volume:         regs.INITIAL_VOLUME(mmu),
            length:         regs.SOUND_LENGTH(mmu),
            timer:          2048 - regs.FREQ(mmu),
            duty_cycle:     0,
            sweep_timer:    regs.SWEEP_TIME(mmu),
            envelope_count: regs.ENVELOPE_SHIFTS(mmu),
            buff:           Vec::with_capacity(BUFF_SIZE),
            sample_counter: 0,
            regs:           regs,
        }
    }

    fn reset(&mut self, mmu: &mut MMU<impl BankController>) {
        self.buff.clear();
        self.frequency = self.regs.FREQ(mmu);
        self.volume = self.regs.INITIAL_VOLUME(mmu);
        self.length = self.regs.SOUND_LENGTH(mmu);
        self.timer = 2048 - self.frequency;
        self.duty_cycle = 0;
        self.sweep_timer = self.regs.SWEEP_TIME(mmu);
        self.envelope_count = self.regs.ENVELOPE_SHIFTS(mmu);
    }

    fn tick(&mut self, mmu: &mut MMU<impl BankController>) {
        // If triggered start.
        if self.regs.INITIAL(mmu) { 
            self.reset(mmu);
            self.regs._INITIAL(mmu, false);
            self.regs._ENABLED(mmu, true);
        }
        if !self.regs.ENABLED(mmu) { return }
        // Update timer and duty cycle
        if self.timer > 0 { self.timer -= 1 };
        if self.timer == 0 {
            self.duty_cycle = (self.duty_cycle + 1) % DUTY_CYCLE_STEPS;
            self.timer = 2048 - self.frequency;
        }
        // Generate sample
        self.sample_counter += 1;
        if self.sample_counter == SAMPLE_APPEND_RATE {
            let is_on  = DUTY_CYCLES[self.regs.WAVE_DUTY(mmu) as usize][self.duty_cycle as usize];
            let sample = if is_on { (i16::max_value()/0xF)*(self.volume as i16) } 
                         else { 0 };
            self.buff.push(sample);
            self.sample_counter = 0;
        }
    }

    fn buffer(&mut self) -> &mut Vec<i16> { &mut self.buff }

    fn length(&mut self, mmu: &mut MMU<impl BankController>) {
        if !self.regs.ENABLED(mmu) { return }
        if self.length > 0 { self.length -= 1; }
        if self.length == 0 {
            if self.regs.COUNTER_CONSECUTIVE_SELECT(mmu) {
                self.regs._ENABLED(mmu, false); 
            }
        }
    }

    fn sweep(&mut self, mmu: &mut MMU<impl BankController>) {
        if !self.regs.ENABLED(mmu) { return }
        self.sweep_timer -= 1;
        if self.sweep_timer == 0 {
            let delta = self.frequency/(2 as u16).pow(self.regs.SWEEP_SHIFTS(mmu) as u32);
            if self.regs.SWEEP_DIRECTION(mmu) { 
                if self.frequency >= delta { self.frequency -= delta; }
            } else if self.frequency + delta > 0x7FF { 
                self.regs._ENABLED(mmu, false);
            } else {
                self.frequency += delta;
            }
            self.sweep_timer = self.regs.SWEEP_TIME(mmu);
        }
    }

    fn envelope(&mut self, mmu: &mut MMU<impl BankController>) {
        if !self.regs.ENABLED(mmu) || self.envelope_count == 0 { return }
        if !self.regs.ENVELOPE_DIRECTION(mmu) {
            if self.volume < 0xF { self.volume += 1; }
        } else {
            if self.volume > 0   { self.volume -= 1 }
        }
        self.envelope_count -= 1;
    }
}

struct WaveRamChannel {
    length: u16,
    frequency: u16,
    timer: u16,
    position_counter: usize,
    sample_counter: u16,
    buff: Vec<i16>
}

impl WaveRamChannel {
    fn new(mmu: &mut MMU<impl BankController>) -> Self {
        Self {
            length : Self::SOUND_LENGTH(mmu),
            frequency: Self::FREQ(mmu),
            timer: 2048 - Self::FREQ(mmu),
            sample_counter: 0,
            position_counter: 0,
            buff: Vec::with_capacity(BUFF_SIZE),
        }
    }

    fn reset(&mut self, mmu: &mut MMU<impl BankController>) {
        //self.buff.clear();
        self.length = Self::SOUND_LENGTH(mmu);
        self.frequency = Self::FREQ(mmu);
        self.timer = (2048 - self.frequency)/2;
    }

    fn tick(&mut self, mmu: &mut MMU<impl BankController>) {
        // If triggered start.
        if Self::INITIAL(mmu) { 
            self.reset(mmu);
            Self::_INITIAL(mmu, false);
            Self::_ENABLED(mmu, true);
        }
        if !Self::ENABLED(mmu) || !Self::OUTPUTTING(mmu) { return }
        // Update timer and position in wave ram
        if self.timer > 0 { self.timer -= 1 };
        if self.timer == 0 {
            self.position_counter = (self.position_counter + 1) % WAVE_RAM_SAMPLE_COUNT;
            self.timer = (2048 - self.frequency)/2;
        }
        // Generate sample
        self.sample_counter += 1;
        if self.sample_counter == SAMPLE_APPEND_RATE {
            let offset = (self.position_counter as u16)/2;
            let sample_byte = mmu.read(WAVE_RAM_BASE + offset);
            let mut volume = if self.position_counter % 2 == 0 { 
                sample_byte >> 4 
            } else { 
                sample_byte & 0xF 
            };
            volume = match Self::OUTPUT_LEVEL(mmu) {
                0 => 0,
                1 => volume,
                2 => volume >> 1,
                3 => volume >> 2,
                x => panic!("Invalid output level {}", x),
            };
            let sample = (i16::max_value()/0xF)*(volume as i16);
            self.buff.push(sample);
            self.sample_counter = 0;
        }
    }

    fn length(&mut self, mmu: &mut MMU<impl BankController>) {
        if !Self::ENABLED(mmu) { return }
        if self.length > 0  { self.length -= 1; }
        if self.length == 0 {
            if Self::COUNTER_CONSECUTIVE_SELECT(mmu) {
                Self::_ENABLED(mmu, false); 
            }
        }
    }
        
    fn buffer(&mut self) -> &mut Vec<i16> { &mut self.buff }

    // NR30 - Sound ON/OFF
    fn OUTPUTTING(mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_30, 7) }
    fn _OUTPUTTING(mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_30, 7, value) }

    // NR31 - Sound Length
    fn SOUND_LENGTH(mmu: &mut MMU<impl BankController>) -> u16 { mmu.read(ioregs::NR_31) as u16 }

    // NR32 - Output level
    fn OUTPUT_LEVEL(mmu: &mut MMU<impl BankController>) -> u8 { (mmu.read(ioregs::NR_32) >> 5) & 3 }

    // NR 33 and NR 34 - frequency
    fn FREQ(mmu: &mut MMU<impl BankController>) -> u16 {
        (((mmu.read(ioregs::NR_34) & 7) as u16) << 8) + mmu.read(ioregs::NR_33) as u16
    }
    fn COUNTER_CONSECUTIVE_SELECT(mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_34, 6) }
    fn INITIAL(mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_34, 7) }
    fn _INITIAL(mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_34, 7, value) }

    // NR52 - Sound ON/OFF
    fn ENABLED(mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_52, 2) }
    fn _ENABLED(mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_52, 2, value) }
}

struct NoiseChannel {
    volume: u16,
    length: u16,
    envelope_count: u8,
    timer: u16,
    sample_counter: u16,
    lsfr: [bool; NOISE_LSFR_SIZE], 
    buff: Vec<i16>
}

impl NoiseChannel {
    fn new(mmu: &mut MMU<impl BankController>) -> Self {
        Self {
            volume: Self::INITIAL_VOLUME(mmu),
            length: Self::SOUND_LENGTH(mmu),
            envelope_count: Self::ENVELOPE_SHIFTS(mmu),
            timer: Self::FREQ_RATIO(mmu) << Self::FREQ_SHIFT_CLOCK(mmu),
            sample_counter: 0,
            lsfr: [false; NOISE_LSFR_SIZE],
            buff: Vec::with_capacity(BUFF_SIZE),
        }
    }

    fn reset(&mut self, mmu: &mut MMU<impl BankController>) {
        self.buff.clear();
        self.volume = Self::INITIAL_VOLUME(mmu);
        self.length = Self::SOUND_LENGTH(mmu);
        self.timer = Self::FREQ_RATIO(mmu) << Self::FREQ_SHIFT_CLOCK(mmu);
        self.envelope_count = Self::ENVELOPE_SHIFTS(mmu);
        self.sample_counter = 0;
    }

    fn tick(&mut self, mmu: &mut MMU<impl BankController>) {
        // If triggered start.
        if Self::INITIAL(mmu) { 
            self.reset(mmu);
            Self::_INITIAL(mmu, false);
            Self::_ENABLED(mmu, true);
        }
        if !Self::ENABLED(mmu) { return }
        // Update timer and position in wave ram
        if self.timer > 0 { self.timer -= 1 };
        if self.timer == 0 {
            let new = self.lsfr[1] ^ self.lsfr[0];
            // Shift it right
            for i in 1..NOISE_LSFR_SIZE { self.lsfr[i-1] = self.lsfr[i]; }
            // Append at the end
            if Self::LSFR_7BIT(mmu) { self.lsfr[(NOISE_LSFR_SIZE-1)/2] = new; } 
            else                    { self.lsfr[NOISE_LSFR_SIZE-1] = new; }
            self.timer = Self::FREQ_RATIO(mmu) << Self::FREQ_SHIFT_CLOCK(mmu);
        }
        // Generate sample
        self.sample_counter += 1;
        if self.sample_counter == SAMPLE_APPEND_RATE {
            let sample = if !self.lsfr[0] { (i16::max_value()/0xF)*(self.volume as i16) }
                         else { 0 };
            self.buff.push(sample);
            self.sample_counter = 0;
        }
    }

    fn length(&mut self, mmu: &mut MMU<impl BankController>) {
        if !Self::ENABLED(mmu) { return }
        if self.length > 0  { self.length -= 1; }
        if self.length == 0 {
            if Self::COUNTER_CONSECUTIVE_SELECT(mmu) {
                Self::_ENABLED(mmu, false); 
            }
        }
    }

    fn envelope(&mut self, mmu: &mut MMU<impl BankController>) {
        if !Self::ENABLED(mmu) || self.envelope_count == 0 { return }
        if !Self::ENVELOPE_DIRECTION(mmu) {
            if self.volume < 0xF { self.volume += 1; }
        } else {
            if self.volume > 0   { self.volume -= 1 }
        }
        self.envelope_count -= 1;
    }

    fn buffer(&mut self) -> &mut Vec<i16> { &mut self.buff }

    // NR 41 - Length register
    fn SOUND_LENGTH(mmu: &mut MMU<impl BankController>) -> u16 { (mmu.read(ioregs::NR_41) & 0x3F) as u16 }

    // NR 42 - Volume Envelope register
    fn ENVELOPE_SHIFTS(mmu: &mut MMU<impl BankController>) -> u8 { mmu.read(ioregs::NR_42) & 7 }
    fn ENVELOPE_DIRECTION(mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_42, 3) }
    fn INITIAL_VOLUME(mmu: &mut MMU<impl BankController>) -> u16 { (mmu.read(ioregs::NR_42) >> 4) as u16 }

    // NR 43 - Frequency config
    fn FREQ_RATIO(mmu: &mut MMU<impl BankController>) -> u16 {
        let x = (mmu.read(ioregs::NR_43) & 7) as u16;
        if x == 0 { 8 }
        else      { 8*x }
    }
    fn LSFR_7BIT(mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_43, 3) }
    fn FREQ_SHIFT_CLOCK(mmu: &mut MMU<impl BankController>) -> u16 {
        (mmu.read(ioregs::NR_43) >> 4) as u16
    }

    // NR 44 - Counter/Consecutive selection and initial flags
    fn COUNTER_CONSECUTIVE_SELECT(mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_44, 6)  }
    fn INITIAL(mmu: &mut MMU<impl BankController>) -> bool {mmu.read_bit(ioregs::NR_44, 7) }
    fn _INITIAL(mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_44, 7, value) }

    // NR52 - Sound ON/OFF
    fn ENABLED(mmu: &mut MMU<impl BankController>) -> bool { mmu.read_bit(ioregs::NR_52, 3) }
    fn _ENABLED(mmu: &mut MMU<impl BankController>, value: bool) { mmu.set_bit(ioregs::NR_52, 3, value) }
}

pub struct APU {
    /* If sequencer_cycle % (1MHz/512Hz) == 0 then sequencer_step increments */
    sequencer_cycle: u16,
    /* Number between 0-7. It wraps around. */
    sequencer_step: u16,
    /* Quadrangular wave patterns with sweep and envelope functions. */
    chan1: SquareWaveChannel<Channel1Regs>,
    chan2: SquareWaveChannel<Channel2Regs>,
    chan3: WaveRamChannel,
    chan4: NoiseChannel,
}

impl <T: BankController>Clocked<T> for APU {
    
    // Can always catchup
    fn next_time(&self, _: &mut MMU<T>) -> u64 { 1 }

    fn step(&mut self, mmu: &mut MMU<T>) { 
        self.chan1.tick(mmu);
        self.chan2.tick(mmu);
        self.chan3.tick(mmu);
        self.chan4.tick(mmu);

        self.sequencer_cycle += 1;
        if self.sequencer_cycle == SEQUENCER_UPDATE_RATE {
            match self.sequencer_step { 0 | 2 | 4 | 6 => {
                    self.chan1.length(mmu);
                    self.chan2.length(mmu);
                    self.chan3.length(mmu);
                    self.chan4.length(mmu);
                }, _ => {},
            };
            match self.sequencer_step { 2 | 6 => {
                    self.chan1.sweep(mmu);
                    // No sweep for chan2, chan3, chan4
                }, _ => {},
            };
            match self.sequencer_step { 7 => {
                    self.chan1.envelope(mmu);
                    self.chan2.envelope(mmu);
                    // Noe envelope for chan3
                    self.chan4.envelope(mmu);
                }, _ => {},
            };

            self.sequencer_cycle = 0;
            self.sequencer_step = (self.sequencer_step + 1) % SEQUENCER_STEP_COUNT;
        }
    }
}

impl APU {
    pub fn new(mmu: &mut MMU<impl BankController>) -> Self {
        Self {
            sequencer_cycle: 0,
            sequencer_step: 0,
            chan1: SquareWaveChannel::new(mmu, Channel1Regs),
            chan2: SquareWaveChannel::new(mmu, Channel2Regs),
            chan3: WaveRamChannel::new(mmu),
            chan4: NoiseChannel::new(mmu),
        }
    }

    /* Is channel conected to terminal 1? */
    pub fn SO1(mmu: &mut MMU<impl BankController>, chan: u8) -> bool {
        if chan > 4  || chan == 0 { return false }
        let chan = chan - 1;
        let nr_51 = mmu.read(ioregs::NR_51);
        (nr_51 & (1 << chan)) != 0
    }
    
    /* Is channel conected to terminal 2? */
    pub fn SO2(mmu: &mut MMU<impl BankController>, chan: u8) -> bool {
        if chan > 4  || chan == 0 { return false }
        let chan = chan - 1;
        let nr_51 = mmu.read(ioregs::NR_51) >> 4;
        (nr_51 & (1 << chan)) != 0
    }

    pub fn chan1_disable(&mut self, mmu: &mut MMU<impl BankController>) { self.chan1.regs._ENABLED(mmu, false); }
    pub fn chan2_disable(&mut self, mmu: &mut MMU<impl BankController>) { self.chan2.regs._ENABLED(mmu, false); }
    pub fn chan3_disable(&mut self, mmu: &mut MMU<impl BankController>) { WaveRamChannel::_ENABLED(mmu, false); }
    pub fn chan4_disable(&mut self, mmu: &mut MMU<impl BankController>) { NoiseChannel::_ENABLED(mmu, false); }

    pub fn chan1_samples(&mut self) -> &mut Vec<i16> { self.chan1.buffer() }
    pub fn chan2_samples(&mut self) -> &mut Vec<i16> { self.chan2.buffer() }
    pub fn chan3_samples(&mut self) -> &mut Vec<i16> { self.chan3.buffer() }
    pub fn chan4_samples(&mut self) -> &mut Vec<i16> { self.chan4.buffer() }

    pub fn chan1_reset(&mut self, mmu: &mut MMU<impl BankController>) { self.chan1.reset(mmu); }
    pub fn chan2_reset(&mut self, mmu: &mut MMU<impl BankController>) { self.chan2.reset(mmu); }
    pub fn chan3_reset(&mut self, mmu: &mut MMU<impl BankController>) { self.chan3.reset(mmu); }
    pub fn chan4_reset(&mut self, mmu: &mut MMU<impl BankController>) {  }
}