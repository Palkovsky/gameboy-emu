pub mod gpu;
pub use gpu::*;

/*
 * Trait representing a clocked device. 
 */ 
trait Clocked {
    pub fn frequency() -> u64;
}