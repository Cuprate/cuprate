#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(usize)]
pub enum RGroupRegisterID {
    R0 = 0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
}

impl RGroupRegisterID {
    pub fn iter() -> impl Iterator<Item = RGroupRegisterID> {
        [
            RGroupRegisterID::R0,
            RGroupRegisterID::R1,
            RGroupRegisterID::R2,
            RGroupRegisterID::R3,
            RGroupRegisterID::R4,
            RGroupRegisterID::R5,
            RGroupRegisterID::R6,
            RGroupRegisterID::R7,
        ]
        .into_iter()
    }
}

#[derive(Debug, Default, Clone)]
pub struct RGroupRegisters([u64; 8]);

impl RGroupRegisters {
    pub fn apply_to_dst(&mut self, dst: &RGroupRegisterID, f: impl FnOnce(u64) -> u64) {
        *self.get_mut(dst) = f(self.get(dst));
    }

    pub fn apply_to_dst_with_src(
        &mut self,
        dst: &RGroupRegisterID,
        src: &RGroupRegisterID,
        f: impl FnOnce(u64, u64) -> u64,
    ) {
        *self.get_mut(dst) = f(self.get(dst), self.get(src));
    }

    pub fn set(&mut self, id: &RGroupRegisterID, val: u64) {
        self.0[*id as usize] = val
    }

    pub fn get(&self, id: &RGroupRegisterID) -> u64 {
        self.0[*id as usize]
    }

    pub fn get_mut(&mut self, id: &RGroupRegisterID) -> &mut u64 {
        &mut self.0[*id as usize]
    }
}
