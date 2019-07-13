// Configurations
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum CPUExecutionMode {
    User,
    FIQ,
    Supervisor,
    Abort,
    IRQ,
    Undefined,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum CPUByteOrder {
    BigEndian,
    LittleEndian,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum CPUInstructionMode {
    ARM32,
    Thumb,
}

// These are the default modes when the GBA is turned on
impl Default for CPUExecutionMode {
    fn default() -> CPUExecutionMode {
        CPUExecutionMode::Supervisor
    }
}

impl Default for CPUByteOrder {
    fn default() -> CPUByteOrder {
        CPUByteOrder::BigEndian
    }
}

impl Default for CPUInstructionMode {
    fn default() -> CPUInstructionMode {
        CPUInstructionMode::ARM32
    }
}
