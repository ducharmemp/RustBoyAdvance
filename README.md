# RustBoyAdvance
Game Boy Advance emulator written in the Rust programming language.

To compile:
	cargo build (cargo is included with the rust compiler)

To run:
	cargo run PATH/TO/ROMS/BIOS.gba PATH/TO/ROMS/test.bin

The debugger is enabled with no way to turn it off (kind of an oversight),
when everything is loaded in, input "r" to advance the program and print out the registers,
"bp $(MEMORY LOCATION)" to set a breakpoint for a certain memory location,
"c" to run the program without stopping.

The version of Rust used to compile this project is 1.8, and can be found at https://www.rust-lang.org/downloads.html