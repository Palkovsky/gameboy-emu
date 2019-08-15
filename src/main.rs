const RAM_SIZE: usize = 0xFFFF;
type Addr = u16;
type Byte = u8;

/*
 * Memory struct represents GB memory map.
 * It is parametrized by Memory Bank Controller for mappers handling.
 */
struct Memory<T: BankController> {
    mbc: T,
    game: Vec<Byte>,
    bytes: Vec<Byte>,
}

impl<T: BankController> Memory<T> {
    pub fn new(mbc: T, game: Vec<Byte>) -> Self {
        Self { mbc: mbc, game: game, bytes: vec![0; RAM_SIZE] }
    }
    pub fn write(&mut self, addr: Addr, byte: Byte) {
        let mbc = &mut self.mbc;
        match mbc.get_addr_type(addr) {
            AddrType::Config => 
                mbc.on_config(addr, byte),
            AddrType::ROMSwap | AddrType::RAMSwap => 
                mbc.on_swap(addr, byte, &self.game, &mut self.bytes),
            AddrType::RAM => 
                self.bytes[addr as usize] = byte,
            AddrType::ROM => 
                panic!("Write to ROM at 0x{:X}", addr),
        }
    }
    pub fn read(&self, addr: Addr) -> Byte {
        self.bytes[addr as usize]
    }
}

/*
 * AddrType is used by BankController to determine address type: wheater it is
 * will change MBC registers or perform bank switching or is just regular writable.
 */
#[derive(Copy, Clone)]
enum AddrType {
    RAM,
    ROM,
    Config,
    ROMSwap,
    RAMSwap,
}

/*
 * BankController trait represents memory mapper interface.
 */
trait BankController {
    /*
     * Checks wheater the addr is special memory region for
     * MBC configuration(setting registers, enabling RAM etc.). 
     */
    fn get_addr_type(&self, addr: Addr) -> AddrType;
    /*
     * Called when get_addr_type() returned Config addr type.
     */
    fn on_config(&mut self, addr: Addr, value: Byte);
    /*
     * Called hen get_addr_type() returns ROMSwap/RAMSwap
     */
    fn on_swap(&mut self, addr: Addr, value: Byte, game: &[Byte], memory: &mut [Byte]);
}

/*
 * Simplest MBC - no switching needed. This implementation assumes that switchable RAM
 * bank(0xA000-0xBFFF) is available.
 */
struct RomOnly {}
impl RomOnly {
    fn new() -> Self { Self {} }
}
impl BankController for RomOnly {
    fn get_addr_type(&self, addr: Addr) -> AddrType {
        if addr < 0x8000 { AddrType::ROM } else { AddrType::RAM }
    }    
    fn on_config(&mut self, _: Addr, _: Byte) {}
    fn on_swap(&mut self, _: Addr, _: Byte, _: &[Byte], _: &mut [Byte]){}
}

struct MBC1 {
    ram_flg: u8, // 0 -> disabled, otherwise -> enabled
    banking_flg: u8, // 0 -> rom, 1 -> ram
}
impl MBC1 {
    fn new() -> Self { Self {ram_flg: 0, banking_flg: 0} }
}
impl BankController for MBC1 {
    fn get_addr_type(&self, addr: Addr) -> AddrType {
        let intervals = [
            (0x0000, 0x1FFF, AddrType::Config), // RAM enable
            (0x6000, 0x7FFF, AddrType::Config), // ROM/RAM banking mode
            (0xA000, 0xBFFF, AddrType::RAMSwap), // RAM Bank swap
            (0x2000, 0x3FFF, AddrType::ROMSwap), // ROM bank swap
            (0x4000, 0x5FFF, if self.banking_flg == 0 { AddrType::ROMSwap } else { AddrType::RAMSwap }),
        ];
        for (start, end, t) in intervals.iter() {
            if addr >= *start && addr <= *end { return *t }
        }
        if addr < 0x8000 { AddrType::ROM } else { AddrType::RAM }
    }    
    fn on_config(&mut self, _: Addr, _: Byte) {}
    fn on_swap(&mut self, _: Addr, _: Byte, _: &[Byte], _: &mut [Byte]){}   
}

fn main() {
    // Mock of game ROM
    let game: Vec<Byte> = vec![0; 2137*2137];
    let cart_header: Vec<Byte> = game.iter()
        .take(0x150).map(|x| *x).collect();
    let cart_body: Vec<Byte> = game.iter()
        .skip(0x150).map(|x| *x).collect();

    let mapper = RomOnly::new();
    let mut memory = Memory::new(mapper, cart_body);

    memory.write(0x8000, 0x69);
    println!("Read: {:x} {:x}", memory.read(0x8000), memory.read(0x8001));
    memory.write(0x0000, 0x69);
}
