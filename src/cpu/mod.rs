pub mod cpu;

// Export the name to cut down on the verbosity
// TODO: Maybe not a good idea, think of a better naming scheme
use cpu::cpu::CPU;

// Crates that need to be used by the CPU.
// Rust's module system is really really bad. It's the only complaint I have so far.
extern crate byteorder;
