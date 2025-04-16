use bicycle_isa::BicycleISA;
use clap::ValueEnum;
use fixed::types::U32F96;

// Because we need to support precision up to 10^-20,
// which is >2^-65
pub type ErrorPrecision = U32F96;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Model {
    timing: TimingModel,
    error: ErrorModel,
}

impl Model {
    pub fn timing(&self, instruction: &BicycleISA) -> u64 {
        self.timing.timing(instruction)
    }

    pub fn instruction_error(&self, instruction: &BicycleISA) -> ErrorPrecision {
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
    pub fn timing(&self, instruction: &BicycleISA) -> u64 {
        match instruction {
            BicycleISA::TGate(_) => self.t_inj,
            BicycleISA::Automorphism(_) => self.shift,
            BicycleISA::Measure(_) => self.inmodule,
            BicycleISA::JointMeasure(_) => self.intermodule,
            _ => unreachable!("Should not have instruction {}", instruction),
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
    pub fn instruction_error(&self, instruction: &BicycleISA) -> ErrorPrecision {
        match instruction {
            BicycleISA::TGate(_) => self.t_inj,
            BicycleISA::Measure(_) => self.inmodule,
            BicycleISA::JointMeasure(_) => self.intermodule,
            BicycleISA::Automorphism(_) => self.shift,
            _ => unreachable!("Should not have instruction {}", instruction),
        }
    }

    pub fn idling_error(&self, cycles: u64, idle_cycles: u64) -> ErrorPrecision {
        (cycles.div_ceil(idle_cycles) as u128) * self.idle
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum ModelChoices {
    Gross1e3,
    Gross1e4,
    TwoGross1e3,
    TwoGross1e4,
}

impl ModelChoices {
    pub fn model(self) -> Model {
        match self {
            Self::Gross1e3 => GROSS_1E3,
            Self::Gross1e4 => GROSS_1E4,
            Self::TwoGross1e3 => TWO_GROSS_1E3,
            Self::TwoGross1e4 => TWO_GROSS_1E4,
        }
    }
}

pub const GROSS_1E3: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1e-6"),
        shift: ErrorPrecision::lit("1e-5"),
        inmodule: ErrorPrecision::lit("1e-5"),
        intermodule: ErrorPrecision::lit("1e-5"),
        t_inj: ErrorPrecision::lit("1e-4"),
    },
    timing: GROSS_TIMING,
};

pub const GROSS_1E4: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1e-11"),
        shift: ErrorPrecision::lit("1e-10"),
        inmodule: ErrorPrecision::lit("1e-9"),
        intermodule: ErrorPrecision::lit("1e-9"),
        t_inj: ErrorPrecision::lit("1e-9"),
    },
    timing: GROSS_TIMING,
};

pub const TWO_GROSS_1E3: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1e-11"),
        shift: ErrorPrecision::lit("1e-10"),
        inmodule: ErrorPrecision::lit("1e-9"),
        intermodule: ErrorPrecision::lit("1e-9"),
        t_inj: ErrorPrecision::lit("1e-10"),
    },
    timing: TWO_GROSS_TIMING,
};

pub const TWO_GROSS_1E4: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1e-20"),
        shift: ErrorPrecision::lit("1e-19"),
        inmodule: ErrorPrecision::lit("1e-18"),
        intermodule: ErrorPrecision::lit("1e-18"),
        t_inj: ErrorPrecision::lit("1e-18"),
    },
    timing: TWO_GROSS_TIMING,
};
