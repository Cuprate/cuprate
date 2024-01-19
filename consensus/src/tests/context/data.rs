use monero_consensus::HardFork;

pub static HFS_2688888_2689608: [(HardFork, HardFork); 720] = include!("./data/hfs_2688888_2689608");

pub static HFS_2678808_2688888: [(HardFork, HardFork); 10080] =
    include!("./data/hfs_2678808_2688888");

pub static BW_2850000_3050000: [(usize, usize); 200_000] = include!("./data/bw_2850000_3050000");
