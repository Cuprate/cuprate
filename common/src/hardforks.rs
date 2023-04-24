use chrono::NaiveDateTime;

use crate::network::Network;

// this function blindly unwraps 
// SAFETY: only call when you know the timestamp is good
fn time_from_timestamp(stamp: i64) -> NaiveDateTime {
    NaiveDateTime::from_timestamp_opt(stamp, 0).unwrap()
}

fn get_hard_forks(network: Network) -> [(u8, u64, NaiveDateTime); 16] {
    match network {
        Network::MainNet => {
            [
                // |    version      |       Height      |       TimeStamp    | *timestamp is when fork height was decided
                (1, 1, time_from_timestamp(1341378000)),
                (2, 1009827, time_from_timestamp(1442763710)),
                (3, 1141317, time_from_timestamp(1458558528)),
                (4, 1220516, time_from_timestamp(1483574400)),
                (5, 1288616, time_from_timestamp(1489520158)),
                (6, 1400000, time_from_timestamp(1503046577)),
                (7, 1546000, time_from_timestamp(1521303150)),
                (8, 1685555, time_from_timestamp(1535889547)),
                (9, 1686275, time_from_timestamp(1535889548)),
                (10, 1788000, time_from_timestamp(1549792439)),
                (11, 1788720, time_from_timestamp(1550225678)),
                (12, 1978433, time_from_timestamp(1571419280)),
                (13, 2210000, time_from_timestamp(1598180817)),
                (14, 2210720, time_from_timestamp(1598180818)),
                (15, 2688888, time_from_timestamp(1656629117)),
                (16, 2689608, time_from_timestamp(1656629118)),
            ]
        }
        Network::TestNet => [
            (1, 1, time_from_timestamp(1341378000)),
            (2, 624634, time_from_timestamp(1445355000)),
            (3, 800500, time_from_timestamp(1472415034)),
            (4, 801219, time_from_timestamp(1472415035)),
            (5, 802660, time_from_timestamp(1472415036 + 86400 * 180)),
            (6, 971400, time_from_timestamp(1501709789)),
            (7, 1057027, time_from_timestamp(1512211236)),
            (8, 1057058, time_from_timestamp(1533211200)),
            (9, 1057778, time_from_timestamp(1533297600)),
            (10, 1154318, time_from_timestamp(1550153694)),
            (11, 1155038, time_from_timestamp(1550225678)),
            (12, 1308737, time_from_timestamp(1569582000)),
            (13, 1543939, time_from_timestamp(1599069376)),
            (14, 1544659, time_from_timestamp(1599069377)),
            (15, 1982800, time_from_timestamp(1652727000)),
            (16, 1983520, time_from_timestamp(1652813400)),
        ],
        Network::StageNet => [
            (1, 1, time_from_timestamp(1341378000)),
            (2, 32000, time_from_timestamp(1521000000)),
            (3, 33000, time_from_timestamp(1521120000)),
            (4, 34000, time_from_timestamp(1521240000)),
            (5, 35000, time_from_timestamp(1521360000)),
            (6, 36000, time_from_timestamp(1521480000)),
            (7, 37000, time_from_timestamp(1521600000)),
            (8, 176456, time_from_timestamp(1537821770)),
            (9, 177176, time_from_timestamp(1537821771)),
            (10, 269000, time_from_timestamp(1550153694)),
            (11, 269720, time_from_timestamp(1550225678)),
            (12, 454721, time_from_timestamp(1571419280)),
            (13, 675405, time_from_timestamp(1598180817)),
            (14, 676125, time_from_timestamp(1598180818)),
            (15, 1151000, time_from_timestamp(1656629117)),
            (16, 1151720, time_from_timestamp(1656629118)),
        ],
    }
}

pub struct HardForks {
    hard_forks: [(u8, u64, NaiveDateTime); 16],
}

impl HardForks {
    pub fn new(network: Network) -> Self {
        HardForks {
            hard_forks: get_hard_forks(network),
        }
    }

    pub fn get_ideal_version_from_height(&self, height: u64) -> u8 {
        for hf in self.hard_forks.iter().rev() {
            if height >= hf.1 {
                return hf.0;
            }
        }
        0
    }

    pub fn get_earliest_ideal_height_for_version(&self, version: u8) -> Option<u64> {
        if self.hard_forks.len() < version as usize {
            None
        } else if version == 0 {
            Some(0)
        } else {
            Some(self.hard_forks[(version - 1) as usize].1)
        }
    }

    pub fn get_ideal_version(&self) -> u8 {
        self.hard_forks.last().expect("This is not empty").0
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Network;

    use super::HardForks;

    const MAIN_NET_FORKS: [u64; 16] = [
        1, 1009827, 1141317, 1220516, 1288616, 1400000, 1546000, 1685555, 1686275, 1788000,
        1788720, 1978433, 2210000, 2210720, 2688888, 2689608,
    ];
    const TEST_NET_FORKS: [u64; 16] = [
        1, 624634, 800500, 801219, 802660, 971400, 1057027, 1057058, 1057778, 1154318, 1155038,
        1308737, 1543939, 1544659, 1982800, 1983520,
    ];
    const STAGE_NET_FORKS: [u64; 16] = [
        1, 32000, 33000, 34000, 35000, 36000, 37000, 176456, 177176, 269000, 269720, 454721,
        675405, 676125, 1151000, 1151720,
    ];

    #[test]
    fn get_ideal_version() {
        let hardforks = HardForks::new(Network::MainNet);

        let version = hardforks.get_ideal_version();
        assert_eq!(version as usize, MAIN_NET_FORKS.len());
        assert_eq!(version as usize, TEST_NET_FORKS.len());
        assert_eq!(version as usize, STAGE_NET_FORKS.len());

        let height = hardforks
            .get_earliest_ideal_height_for_version(version)
            .unwrap();
        let got_version = hardforks.get_ideal_version_from_height(height);
        assert_eq!(version, got_version);
    }

    #[test]
    fn get_earliest_ideal_height_for_version_mainnet() {
        let hardforks = HardForks::new(Network::MainNet);

        for (height, version) in MAIN_NET_FORKS.iter().zip(1..MAIN_NET_FORKS.len() as u8) {
            assert_eq!(
                hardforks
                    .get_earliest_ideal_height_for_version(version)
                    .unwrap(),
                *height
            );
            assert_eq!(
                hardforks
                    .get_earliest_ideal_height_for_version(version)
                    .unwrap(),
                *height
            );
        }
        assert!(hardforks
            .get_earliest_ideal_height_for_version(MAIN_NET_FORKS.len() as u8 + 1)
            .is_none())
    }

    #[test]
    fn get_earliest_ideal_height_for_version_testnet() {
        let hardforks = HardForks::new(Network::TestNet);

        for (height, version) in TEST_NET_FORKS.iter().zip(1..TEST_NET_FORKS.len() as u8) {
            assert_eq!(
                hardforks
                    .get_earliest_ideal_height_for_version(version)
                    .unwrap(),
                *height
            );
            assert_eq!(
                hardforks
                    .get_earliest_ideal_height_for_version(version)
                    .unwrap(),
                *height
            );
        }
        assert!(hardforks
            .get_earliest_ideal_height_for_version(TEST_NET_FORKS.len() as u8 + 1)
            .is_none())
    }

    #[test]
    fn get_earliest_ideal_height_for_version_stagenet() {
        let hardforks = HardForks::new(Network::StageNet);

        for (height, version) in STAGE_NET_FORKS.iter().zip(1..STAGE_NET_FORKS.len() as u8) {
            assert_eq!(
                hardforks
                    .get_earliest_ideal_height_for_version(version)
                    .unwrap(),
                *height
            );
            assert_eq!(
                hardforks
                    .get_earliest_ideal_height_for_version(version)
                    .unwrap(),
                *height
            );
        }
        assert!(hardforks
            .get_earliest_ideal_height_for_version(STAGE_NET_FORKS.len() as u8 + 1)
            .is_none())
    }

    #[test]
    fn get_ideal_version_from_height_mainnet() {
        let hardforks = HardForks::new(Network::MainNet);

        for (height, version) in MAIN_NET_FORKS.iter().zip(1..MAIN_NET_FORKS.len() as u8) {
            assert_eq!(hardforks.get_ideal_version_from_height(*height), version);
            assert_eq!(
                hardforks.get_ideal_version_from_height(*height - 1),
                version - 1
            );
        }
    }

    #[test]
    fn get_ideal_version_from_height_testnet() {
        let hardforks = HardForks::new(Network::TestNet);

        for (height, version) in TEST_NET_FORKS.iter().zip(1..TEST_NET_FORKS.len() as u8) {
            assert_eq!(hardforks.get_ideal_version_from_height(*height), version);
            assert_eq!(
                hardforks.get_ideal_version_from_height(*height - 1),
                version - 1
            );
        }
    }

    #[test]
    fn get_ideal_version_from_height_stagenet() {
        let hardforks = HardForks::new(Network::StageNet);

        for (height, version) in STAGE_NET_FORKS.iter().zip(1..STAGE_NET_FORKS.len() as u8) {
            assert_eq!(hardforks.get_ideal_version_from_height(*height), version);
            assert_eq!(
                hardforks.get_ideal_version_from_height(*height - 1),
                version - 1
            );
        }
    }
}
