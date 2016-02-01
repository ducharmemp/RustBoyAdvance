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

use super::byteorder::BigEndian;

#[allow(dead_code)]
enum CPUExecutionMode {
	User,
	FIQ,
	Supervisor,
	Abort,
	IRQ,
	Undefined
}

#[allow(dead_code)]
enum CPUByteOrder {
	BigEndian,
	LittleEndian
}

#[allow(dead_code)]
enum CPUInstructionMode {
	ARM32,
	Thumb
}

impl Default for CPUExecutionMode {
	fn default() -> CPUExecutionMode {CPUExecutionMode::Supervisor}
}

impl Default for CPUByteOrder {
	fn default() -> CPUByteOrder {CPUByteOrder::BigEndian}
}

impl Default for CPUInstructionMode {
	fn default() -> CPUInstructionMode {CPUInstructionMode::ARM32}
}

#[derive(Default)]
pub struct CPU {
	// unbanked registers r0-r7
	gen_registers: [u32; 8],

	// Banked registers, depend on the current mode
	// r8-r14

	// Current program status register
	cpsr: u32,
	status: u32,

	execution_mode: CPUExecutionMode,
	bytemode: CPUByteOrder,
	instruction_mode: CPUInstructionMode,
}

impl CPU {
	pub fn new() -> CPU {
		CPU::default()
	}

	pub fn power_on_reset(&mut self) {
		self.execution_mode = CPUExecutionMode::Supervisor;
		self.bytemode = CPUByteOrder::BigEndian;
		self.instruction_mode = CPUInstructionMode::ARM32;
	}

	fn run_arm(&mut self) {

	}

	fn run_thumb(&mut self) {

	}

	pub fn decode_instruction_32(&mut self, ins: u32) {
		let cond = 	0xf << 28;
		let i_bit = 0x1 << 25;
		let op = 0xf << 20;
		let s_bit = 0x1 << 19;

		// TODO: Rename these values to some sort of enumeration or something
		match (ins & cond) >> 28 {
			0b0000 => {
				// Z set
			},
			0b0001 => {
				// Z clear
			},
			0b0010 => {
				// C set
			},
			0b0011 => {
				// C clear
			},
			0b0100 => {
				// N set
			},
			0b0101 => {
				// N clear
			},
			0b0110 => {
				// V set
			},
			0b0111 => {
				// V clear
			},
			0b1000 => {
				// C set and Z clear
			},
			0b1001 => {
				// C clear or Z set
			},
			0b1010 => {
				// N set and V set, or N clear and V clear (N == V)
			},
			0b1011 => {
				// N != V
			},
			0b1100 => {
				// Z == 0, N == V
			},
			0b1101 => {
				// Z == 1 or N != V
			},
			0b1110 => {
				// Unconditional
			},
			0b1111 => {
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

	pub fn run(&mut self) {
		loop {
			let mode = self.status & 0x84;
			if mode == 0x0 {
				// ARM

			} else if mode == 0x4 {
				// THUMB
			} else {
				break;
			}
		}
	}
}
