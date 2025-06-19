// (C) Copyright IBM 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use bicycle_isa::BicycleISA;
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

    pub fn idling_error(&self, time: u64) -> (u64, ErrorPrecision) {
        self.error.idling_error(time, self.timing.idle)
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
            BicycleISA::Automorphism(_) => 2 * self.shift,
            BicycleISA::Measure(_) => self.inmodule,
            BicycleISA::JointMeasure(_) => self.intermodule,
            _ => unreachable!("Should not have instruction {}", instruction),
        }
    }
}

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
            BicycleISA::Automorphism(_) => 2 * self.shift,
            _ => unreachable!("Should not have instruction {}", instruction),
        }
    }

    pub fn idling_error(&self, time: u64, idle_cycles: u64) -> (u64, ErrorPrecision) {
        let idle_cycles = time.div_ceil(idle_cycles);
        let idle_error = (idle_cycles as u128) * self.idle;
        (idle_cycles, idle_error)
    }
}

pub const GROSS_1E3: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1.61e-9"),
        shift: ErrorPrecision::lit("4.01e-7"),
        inmodule: ErrorPrecision::lit("1.11e-5"),
        intermodule: ErrorPrecision::lit("2.01e-3"),
        t_inj: ErrorPrecision::lit("2.01e-3"),
    },
    timing: TimingModel {
        idle: 8,
        shift: 12,
        inmodule: 120,
        intermodule: 120,
        t_inj: 351 + 120,
    },
};

pub const GROSS_1E4: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("1.44e-15"),
        shift: ErrorPrecision::lit("6.07e-14"),
        inmodule: ErrorPrecision::lit("1.01e-09"),
        intermodule: ErrorPrecision::lit("4.81e-8"),
        t_inj: ErrorPrecision::lit("8.79e-7"),
    },
    timing: TimingModel {
        idle: 8,
        shift: 12,
        inmodule: 120,
        intermodule: 120,
        t_inj: 73 + 120,
    },
};

pub const TWO_GROSS_1E3: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("8.20e-21"),
        shift: ErrorPrecision::lit("3.25e-15"),
        inmodule: ErrorPrecision::lit("1e-11"),
        intermodule: ErrorPrecision::lit("1e-9"),
        t_inj: ErrorPrecision::lit("2.10e-8"),
    },
    timing: TimingModel {
        idle: 8,
        shift: 12,
        inmodule: 216,
        intermodule: 216,
        t_inj: 2167 + 216,
    },
};

pub const TWO_GROSS_1E4: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("5.29e-39"),
        shift: ErrorPrecision::lit("1.34e-37"),
        inmodule: ErrorPrecision::lit("1e-20"),
        intermodule: ErrorPrecision::lit("1e-18"),
        t_inj: ErrorPrecision::lit("1e-18"),
    },
    timing: TimingModel {
        idle: 8,
        shift: 12,
        inmodule: 216,
        intermodule: 216,
        t_inj: 407 + 216,
    },
};

pub const FAKE_SLOW: Model = Model {
    error: ErrorModel {
        idle: ErrorPrecision::lit("0"),
        shift: ErrorPrecision::lit("0"),
        inmodule: ErrorPrecision::lit("0"),
        intermodule: ErrorPrecision::lit("0"),
        t_inj: ErrorPrecision::lit("0"),
    },
    timing: TimingModel {
        idle: 8,
        shift: 12,
        inmodule: 216,
        intermodule: 216,
        t_inj: 2167 + 216,
    },
};
