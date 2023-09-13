use crate::config::RANDOMX_SUPERSCALAR_LATENCY;

/// Max cycles + highest amount of cycles on a macro op.
const CYCLE_MAP_SIZE: usize = RANDOMX_SUPERSCALAR_LATENCY + 4;

pub(crate) enum SlotLen {
    L3,
    L4,
    L7,
    L8,
    L9,
    L10,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ExecutionPort {
    P0,
    P1,
    P5,
}

enum AllowedPorts {
    One(ExecutionPort),
    Two(ExecutionPort, ExecutionPort),
    All,
}

impl AllowedPorts {
    fn port_allowed(&self, port: &ExecutionPort) -> bool {
        match self {
            AllowedPorts::One(allowed_port) => allowed_port == port,
            AllowedPorts::Two(allowed_port_1, allowed_port_2) => {
                allowed_port_1 == port || allowed_port_2 == port
            }
            AllowedPorts::All => true,
        }
    }
}

pub enum MacroOp {
    SUB_RR,
    XOR_RR,
    LEA_SIB,
    IMUL_RR { dependant: bool },
    ROR_RI,
    ADD_RI,
    XOR_RI,
    MOV_RR,
    MUL_R,
    IMUL_R,
    MOV_RI,
}

impl MacroOp {
    pub fn cycles_to_complete(&self) -> usize {
        match self {
            MacroOp::SUB_RR => 1,
            MacroOp::XOR_RR => 1,
            MacroOp::LEA_SIB => 1,
            MacroOp::IMUL_RR { .. } => 3,
            MacroOp::ROR_RI => 1,
            MacroOp::ADD_RI => 1,
            MacroOp::XOR_RI => 1,
            MacroOp::MOV_RR => 0,
            MacroOp::MUL_R => 4,
            MacroOp::IMUL_R => 4,
            MacroOp::MOV_RI => 1,
        }
    }

    pub fn can_be_eliminated(&self) -> bool {
        self.micro_ops_needed() == 0
    }

    pub fn is_dependant_on_last_op(&self) -> bool {
        match self {
            MacroOp::IMUL_RR { dependant } => *dependant,
            _ => false,
        }
    }

    pub fn micro_ops_needed(&self) -> usize {
        match self {
            MacroOp::SUB_RR => 1,
            MacroOp::XOR_RR => 1,
            MacroOp::LEA_SIB => 1,
            MacroOp::IMUL_RR { .. } => 1,
            MacroOp::ROR_RI => 1,
            MacroOp::ADD_RI => 1,
            MacroOp::XOR_RI => 1,
            MacroOp::MOV_RR => 0,
            MacroOp::MUL_R => 2,
            MacroOp::IMUL_R => 2,
            MacroOp::MOV_RI => 1,
        }
    }

    fn allowed_execution_ports(&self, micro_op_index: usize) -> AllowedPorts {
        match self {
            MacroOp::SUB_RR => AllowedPorts::All,
            MacroOp::XOR_RR => AllowedPorts::All,
            MacroOp::LEA_SIB => AllowedPorts::Two(ExecutionPort::P0, ExecutionPort::P1),
            MacroOp::IMUL_RR { .. } => AllowedPorts::One(ExecutionPort::P1),
            MacroOp::ROR_RI => AllowedPorts::Two(ExecutionPort::P0, ExecutionPort::P5),
            MacroOp::ADD_RI => AllowedPorts::All,
            MacroOp::XOR_RI => AllowedPorts::All,
            MacroOp::MOV_RR => panic!("No execution units needed for MOV_RR"),
            MacroOp::MUL_R => match micro_op_index {
                0 => AllowedPorts::One(ExecutionPort::P1),
                1 => AllowedPorts::One(ExecutionPort::P5),
                _ => panic!("no execution port at that index"),
            },
            MacroOp::IMUL_R => match micro_op_index {
                0 => AllowedPorts::One(ExecutionPort::P1),
                1 => AllowedPorts::One(ExecutionPort::P5),
                _ => panic!("no execution port at that index"),
            },
            MacroOp::MOV_RI => AllowedPorts::All,
        }
    }
}

/// Represents the ports availability during a single cycle.
#[derive(Debug, Default, Copy, Clone)]
struct CycleSchedule {
    p0: bool,
    p1: bool,
    p5: bool,
}

impl CycleSchedule {
    fn space_for_micro_op(&self, allowed_ports: &AllowedPorts) -> Option<ExecutionPort> {
        if !self.p5 && allowed_ports.port_allowed(&ExecutionPort::P5) {
            Some(ExecutionPort::P5)
        } else if !self.p0 && allowed_ports.port_allowed(&ExecutionPort::P0) {
            Some(ExecutionPort::P0)
        } else if !self.p1 && allowed_ports.port_allowed(&ExecutionPort::P1) {
            Some(ExecutionPort::P1)
        } else {
            None
        }
    }

    fn set_port_busy(&mut self, port: ExecutionPort) {
        match port {
            ExecutionPort::P0 => self.p0 = true,
            ExecutionPort::P1 => self.p1 = true,
            ExecutionPort::P5 => self.p5 = true,
        }
    }
}

pub(crate) struct MacroOpOpportunity {
    cycle: usize,
    micro_port_0: Option<ExecutionPort>,
    micro_port_1: Option<ExecutionPort>,
}

impl MacroOpOpportunity {
    pub fn cycle(&self) -> usize {
        self.cycle
    }
}

#[derive(Debug)]
pub(crate) struct ProgramSchedule {
    ports_schedule: [CycleSchedule; CYCLE_MAP_SIZE],
    full: bool,
}

impl Default for ProgramSchedule {
    fn default() -> Self {
        Self {
            ports_schedule: [CycleSchedule::default(); CYCLE_MAP_SIZE],
            full: false,
        }
    }
}

impl ProgramSchedule {
    pub fn set_full(&mut self) {
        self.full = true;
    }

    pub fn is_full(&self) -> bool {
        self.full
    }

    pub fn schedule_macro_op_at_earliest(
        &mut self,
        op: &MacroOp,
        cycle: usize,
        last_op_completes_at: usize,
    ) -> Option<usize> {
        let opportunity = self.earliest_cycle_for_macro_op(op, cycle, last_op_completes_at)?;
        let cycle = opportunity.cycle();
        if let Some(port0) = opportunity.micro_port_0 {
            self.schedule_micro_op(cycle, port0);
            if let Some(port1) = opportunity.micro_port_1 {
                self.schedule_micro_op(cycle, port1);
            };
        };

        Some(cycle)
    }

    pub fn earliest_cycle_for_macro_op(
        &mut self,
        op: &MacroOp,
        cycle: usize,
        last_op_completes_at: usize,
    ) -> Option<MacroOpOpportunity> {
        let mut cycle = if op.is_dependant_on_last_op() {
            cycle.max(last_op_completes_at)
        } else {
            cycle
        };

        if op.can_be_eliminated() {
            return Some(MacroOpOpportunity {
                cycle,
                micro_port_0: None,
                micro_port_1: None,
            });
        }

        match op.micro_ops_needed() {
            0 => Some(MacroOpOpportunity {
                cycle,
                micro_port_0: None,
                micro_port_1: None,
            }),
            1 => self
                .earliest_cycle_for_mirco_op(&op.allowed_execution_ports(0), cycle)
                .map(|(cycle, micro_port_0)| MacroOpOpportunity {
                    cycle,
                    micro_port_0: Some(micro_port_0),
                    micro_port_1: None,
                }),
            2 => {
                // both ops must happen in the same cycle
                let allowed_0 = op.allowed_execution_ports(0);
                let allowed_1 = op.allowed_execution_ports(1);

                while cycle < CYCLE_MAP_SIZE {
                    let (min_0_cycle, port_0) =
                        self.earliest_cycle_for_mirco_op(&allowed_0, cycle)?;
                    let (min_1_cycle, port_1) =
                        self.earliest_cycle_for_mirco_op(&allowed_1, cycle)?;

                    if min_0_cycle == min_1_cycle {
                        return Some(MacroOpOpportunity {
                            cycle: min_0_cycle,
                            micro_port_0: Some(port_0),
                            micro_port_1: Some(port_1),
                        });
                    } else {
                        cycle += 1;
                    }
                }
                None
            }
            _ => unreachable!(),
        }
    }

    fn schedule_micro_op_at_earliest(
        &mut self,
        allowed_ports: &AllowedPorts,
        cycle: usize,
    ) -> Option<usize> {
        let (cycle, port) = self.earliest_cycle_for_mirco_op(allowed_ports, cycle)?;
        self.schedule_micro_op(cycle, port);
        Some(cycle)
    }

    fn schedule_micro_op(&mut self, cycle: usize, port: ExecutionPort) {
        self.ports_schedule[cycle].set_port_busy(port)
    }

    fn earliest_cycle_for_mirco_op(
        &mut self,
        allowed_ports: &AllowedPorts,
        cycle: usize,
    ) -> Option<(usize, ExecutionPort)> {
        for (cycle, cycle_schedule) in self.ports_schedule.iter().enumerate().skip(cycle) {
            if let Some(port) = cycle_schedule.space_for_micro_op(allowed_ports) {
                return Some((cycle, port));
            }
        }
        self.full = true;
        None
    }
}
