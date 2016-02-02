/*
Interrupt Vector Table
Address		Exception	Mode on entry
0x00000000	Reset		Supervisor
0x00000004	Undefined	Undefined
0x00000008	Software	Supervisor
0x0000000C	Abort pref	Abort
0x00000010	Abort data	Abort
0x00000014	Reserved
0x00000018	IRQ			IRQ
0x0000001C	FIQ			FIQ

If multiple exceptions occur simultaneously, prioritize:
	- Reset
	- Data Abort
	- FIQ
	- IRQ
	- Prefetch Abort
	- Undefined Instruction, Software Interrupt
*/
/*
Memory map thanks to http://problemkaputt.de/gbatek.htm#gbamemorymap
*/
// Internal memory
const SYS_MAX_MEMORY: usize = 0x0E00FFFF;

fn bios_memory (x: u32)->bool {0x00000000 <= x && x <= 0x00003FFF}
fn wramb_memory (x: u32)->bool {0x02000000 <= x && x <= 0x0203FFFF}
fn wramc_memory (x: u32)->bool {0x03000000 <= x && x<= 0x03007FFF}
fn io_memory (x: u32)->bool {0x04000000 <= x && x <= 0x040003FE}

// Display Memory
fn obj_pallet_memory (x: u32)->bool {0x05000000 <= x && x <= 0x050003FF}
fn vram_memory (x: u32)->bool {0x06000000 <= x && x <= 0x06017FFF}
fn obj_attr_memory (x: u32)->bool {0x07000000 <= x && x <= 0x070003FF}

// Game Pak Memory
fn wait_0_memory (x: u32)->bool {0x08000000 <= x && x <= 0x09FFFFFF}
fn wait_1_memory (x: u32)->bool {0x0A000000 <= x && x<= 0x0BFFFFFF}
fn wait_2_memory (x: u32)->bool {0x0C000000 <= x && x <= 0x0DFFFFFF}
fn gp_sram_memory (x: u32)->bool {0x0E000000 <= x && x <= SYS_MAX_MEMORY}

fn build_system_ram(bios_rom: Vec<u8>, game_rom: Vec<u8>) -> Vec<u8> {

    let mut ram = vec![0; SYS_MAX_MEMORY];
    ram.extend_from_slice(&bios_rom);
    let mut writer = Cursor::new(ram);

    // Write the game into our RAM vector
    writer.seek(SeekFrom::Start(0x08000000));
    writer.write_all(&game_rom);
    writer.into_inner()
}

// CPSR Flags
const N_FLAG: u32 = 0x80000000;
const Z_FLAG: u32 = 0x40000000;
const C_FLAG: u32 = 0x20000000;
const V_FLAG: u32 = 0x10000000;

const IRQ_DISABLE: u32 = 0x80;
const FIQ_DISABLE: u32 = 0x40;
const STATE_BIT: u32 = 0x20;

const MODE_BITS: u32 = 0xF;

// Configurations
#[allow(dead_code)]
#[derive(Debug)]
enum CPUExecutionMode {
	User,
	FIQ,
	Supervisor,
	Abort,
	IRQ,
	Undefined
}

#[allow(dead_code)]
#[derive(Debug)]
enum CPUByteOrder {
	BigEndian,
	LittleEndian
}

#[allow(dead_code)]
#[derive(Debug)]
enum CPUInstructionMode {
	ARM32,
	Thumb
}

// These are the default modes when the GBA is turned on
impl Default for CPUExecutionMode {
	fn default() -> CPUExecutionMode {CPUExecutionMode::Supervisor}
}

impl Default for CPUByteOrder {
	fn default() -> CPUByteOrder {CPUByteOrder::BigEndian}
}

impl Default for CPUInstructionMode {
	fn default() -> CPUInstructionMode {CPUInstructionMode::ARM32}
}

#[derive(Debug, Default)]
pub struct CPU {
	// unbanked registers r0-r7
	gen_registers: [u32; 8],

	// Banked registers, depend on the current mode
	// r8-r14
	hi_registers_user: [u32; 6],
	hi_registers_supervisor: [u32; 6],
	hi_registers_irq: [u32; 6],
	hi_registers_fiq: [u32; 6],

	// Current program status register
	cpsr: u32,
	status: u32,

	// CPU Configurations
	execution_mode: CPUExecutionMode,
	bytemode: CPUByteOrder,
	instruction_mode: CPUInstructionMode,
}

impl CPU {
	pub fn new() -> CPU {
		CPU::default()
	}

	pub fn decode_instruction_32(&mut self, ins: u32) {
		let condition = ins >> 28;

		// This gets us the 8 bits after the condition and ors it with the 2nd 4 bits of the instruction
		let opcode = ins>>16 & 0xFF0 | ins >> 4 & 0x00F;

		/* VisualBoyAdvance sets up a global table of function pointers for looking up the instruction
		while I would do this in python, the table is 4096 different pointers and that seems like a bit
		overkill and not the best way.

		On the other hand, it's really performant and once I write it once I won't have to worry
		about changing it ever again.

		I don't want to write that many instructions in the impl of the CPU, and I definitely don't
		know how I'm going to keep track of RAM during all of this. I refuse to have it be a global,
		it belongs to the GameBoy itself.
		*/

		// TODO: Rename these values to some sort of enumeration or something
		// Match the condition code
		match condition {
			0x0 => {
				// Z set
			},
			0x1 => {
				// Z clear
			},
			0x2 => {
				// C set
			},
			0x3 => {
				// C clear
			},
			0x4 => {
				// N set
			},
			0x5 => {
				// N clear
			},
			0x6 => {
				// V set
			},
			0x7 => {
				// V clear
			},
			0x8 => {
				// C set and Z clear
			},
			0x9 => {
				// C clear or Z set
			},
			0x10 => {
				// N set and V set, or N clear and V clear (N == V)
			},
			0xA => {
				// N != V
			},
			0xB => {
				// Z == 0, N == V
			},
			0xC => {
				// Z == 1 or N != V
			},
			0xD => {
				// Unconditional
			},
			0xE => {
				/* Unconditional instruction space
				CPS/SETEND
				PLD
				RFE
				SRS
				BLX
				MCRR2
				MRRC2
				STC2
				LDC2
				CDP2
				MCR2
				MRC2
				*/
			},
			_ => panic!("What in the hell happened.")
		}
	}

	fn decode_instruction_16(&mut self, ins: u16) {

	}

	fn run_arm(&mut self) {

	}

	fn run_thumb(&mut self) {

	}

	pub fn run(&mut self) {
		loop {
			let ins = self.get_next_instruction()

			match self.execution_mode {
				CPUInstructionMode::ARM32 => self.run_arm(),
				CPUInstructionMode::Thumb => self.run_thumb(),
				_ => panic!("Invalid mode for instructions")
			}
		}
	}
}
