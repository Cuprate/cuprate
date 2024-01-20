use randomx_rs::{RandomXCache, RandomXError, RandomXFlag, RandomXVM as VMInner};
use thread_local::ThreadLocal;

use monero_consensus::blocks::RandomX;

pub struct RandomXVM {
    vms: ThreadLocal<VMInner>,
    cache: RandomXCache,
    flags: RandomXFlag,
}

impl RandomXVM {
    pub fn new(seed: [u8; 32]) -> Result<Self, RandomXError> {
        let flags = RandomXFlag::get_recommended_flags();

        let cache = RandomXCache::new(flags, &seed)?;

        Ok(RandomXVM {
            vms: ThreadLocal::new(),
            cache,
            flags,
        })
    }
}

impl RandomX for RandomXVM {
    type Error = RandomXError;

    fn calculate_hash(&self, buf: &[u8]) -> Result<[u8; 32], Self::Error> {
        self.vms
            .get_or_try(|| VMInner::new(self.flags, Some(self.cache.clone()), None))?
            .calculate_hash(buf)
            .map(|out| out.try_into().unwrap())
    }
}
