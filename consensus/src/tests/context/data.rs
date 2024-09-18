use cuprate_consensus_rules::HardFork;

pub(crate) static HFS_2688888_2689608: [(HardFork, HardFork); 720] =
    include!("./data/hfs_2688888_2689608");

pub(crate) static HFS_2678808_2688888: [(HardFork, HardFork); 10080] =
    include!("./data/hfs_2678808_2688888");

pub(crate) static BW_2850000_3050000: [(usize, usize); 200_000] =
    include!("./data/bw_2850000_3050000");

pub(crate) static DIF_3000000_3002000: [(u128, u64); 2000] = include!("./data/dif_3000000_3002000");
