mod cpu;
mod executor;
mod generator;
mod instructions;

use crate::blake2_generator::Blake2Generator;

use crate::registers::RGroupRegisterID;
use generator::generate;
use instructions::ScalarInstruction;

pub(crate) struct SSProgram {
    program: Vec<ScalarInstruction>,
    reg_with_max_latency: RGroupRegisterID,
}

impl SSProgram {
    pub fn generate(gen: &mut Blake2Generator) -> Self {
        generate(gen)
    }
}
