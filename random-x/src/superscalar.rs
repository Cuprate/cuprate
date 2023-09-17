mod cpu;
mod executor;
mod generator;
mod instructions;
mod program;

use crate::blake2_generator::Blake2Generator;

use crate::registers::{RGroupRegisterID, RGroupRegisters};
use executor::execute;
use generator::generate;
use instructions::ScalarInstruction;

#[derive(Debug)]
pub(crate) struct SSProgram {
    program: Vec<ScalarInstruction>,
    reg_with_max_latency: RGroupRegisterID,
}

impl SSProgram {
    pub fn generate(gen: &mut Blake2Generator) -> Self {
        generate(gen)
    }

    pub fn execute(&self, registers: &mut RGroupRegisters) {
        execute(&self.program, registers)
    }

    pub fn reg_with_max_latency(&self) -> RGroupRegisterID {
        self.reg_with_max_latency
    }
}
