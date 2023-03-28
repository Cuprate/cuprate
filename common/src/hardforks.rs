use chrono::NaiveDateTime;

use crate::network::Network;


pub struct HardForks {
    network: Network
}

// this function blindly unwraps only call when you know the timestamp is good
fn time_from_timestamp(stamp: i64) -> NaiveDateTime {
    NaiveDateTime::from_timestamp_opt(stamp, 0).unwrap()
}

impl HardForks {
    pub fn new(network: Network) -> Self{
        HardForks { network }
    }

    pub fn get_hard_forks(&self) -> Vec<(u8, u64, NaiveDateTime)> {
        match self.network {
            Network::MainNet => {
                [   // |    version      |       Height      |       TimeStamp    | *timestamp is when fork height was decided 
                    (1, 1, time_from_timestamp(1341378000)),
                    (2, 1009827, time_from_timestamp(1442763710)),
                    (3, 1141317, time_from_timestamp(1458558528)),
                    (4, 1141317, time_from_timestamp(1483574400)),
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

                ].to_vec()
            }
            _ => todo!()
        }
    }
    
}



#[test]
fn test() {
    println!("hardfork v1: {:?}", HardForks::new(Network::MainNet).get_hard_forks()[15].2);
}