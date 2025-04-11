use fixed::types::U32F96;
use pbc_gross::operation::Instruction;

// Because we need to support precision up to 10^-20,
// which is >2^-65
pub type ErrorPrecision = U32F96;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Model {
    timing: TimingModel,
    error: ErrorModel,
}

impl Model {
    pub fn timing(&self, instruction: &Instruction) -> u64 {
        self.timing.timing(instruction)
    }

    pub fn instruction_error(&self, instruction: &Instruction) -> ErrorPrecision {
        self.error.instruction_error(instruction)
    }

    pub fn idling_error(&self, cycles: u64) -> ErrorPrecision {
        self.error.idling_error(cycles, self.timing.idle)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct TimingModel {
    idle: u64,
    shift: u64,
    inmodule: u64,
    intermodule: u64,
    t_inj: u64,
}

impl TimingModel {
    /// Time it takes to perform an instruction
    pub fn timing(&self, instruction: &Instruction) -> u64 {
        match instruction {
            Instruction::Rotation(_) => self.t_inj,
            Instruction::Automorphism(_) => self.shift,
            Instruction::Measure(_) => self.inmodule,
            Instruction::JointMeasure(_) => self.intermodule,
        }
    }
}

const GROSS_TIMING: TimingModel = TimingModel {
    idle: 8,
    shift: 16,
    inmodule: 101,
    intermodule: 101,
    t_inj: 100 + 102,
};
const TWO_GROSS_TIMING: TimingModel = TimingModel {
    idle: 8,
    shift: 16,
    inmodule: 173,
    intermodule: 173,
    t_inj: 100 + 174,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct ErrorModel {
    idle: ErrorPrecision,
    shift: ErrorPrecision,
    inmodule: ErrorPrecision,
    intermodule: ErrorPrecision,
    t_inj: ErrorPrecision,
}

impl ErrorModel {
    pub fn instruction_error(&self, instruction: &Instruction) -> ErrorPrecision {
        match instruction {
            Instruction::Rotation(_) => self.t_inj,
            Instruction::Measure(_) => self.inmodule,
            Instruction::JointMeasure(_) => self.intermodule,
            Instruction::Automorphism(_) => self.shift,
        }
    }

    pub fn idling_error(&self, cycles: u64, idle_cycles: u64) -> ErrorPrecision {
        (cycles.div_ceil(idle_cycles) as u128) * self.idle
    }
}

pub const GROSS_10E3: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1e-6"),
        shift: ErrorPrecision::lit("1e-5"),
        inmodule: ErrorPrecision::lit("1e-4"),
        intermodule: ErrorPrecision::lit("1e-4"),
        t_inj: ErrorPrecision::lit("1e-4"),
    },
    timing: GROSS_TIMING,
};

pub const GROSS_10E4: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1e-11"),
        shift: ErrorPrecision::lit("1e-10"),
        inmodule: ErrorPrecision::lit("1e-9"),
        intermodule: ErrorPrecision::lit("1e-9"),
        t_inj: ErrorPrecision::lit("1e-9"),
    },
    timing: GROSS_TIMING,
};

pub const TWO_GROSS_10E3: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1e-11"),
        shift: ErrorPrecision::lit("1e-10"),
        inmodule: ErrorPrecision::lit("1e-9"),
        intermodule: ErrorPrecision::lit("1e-9"),
        t_inj: ErrorPrecision::lit("1e-10"),
    },
    timing: TWO_GROSS_TIMING,
};

pub const TWO_GROSS_10E4: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1e-20"),
        shift: ErrorPrecision::lit("1e-19"),
        inmodule: ErrorPrecision::lit("1e-18"),
        intermodule: ErrorPrecision::lit("1e-18"),
        t_inj: ErrorPrecision::lit("1e-18"),
    },
    timing: TWO_GROSS_TIMING,
};
