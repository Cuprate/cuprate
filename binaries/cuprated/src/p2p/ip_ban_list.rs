use std::collections::HashSet;
use crate::config::Config;
use cuprate_address_book::{BanList, IpBanList, IpSubnet};
use cuprate_p2p_core::NetZoneAddress;
use cuprate_rpc_types::RpcCallValue;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tracing::instrument;

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct BanListFile {
    #[serde(serialize_with = "serialize_as_string")]
    #[serde(deserialize_with = "deserialize_from_string")]
    subnets: HashSet<IpSubnet>,
    ips: Vec<IpAddr>,
}

impl BanListFile {
    pub fn load_ban_list(config: &Config) -> BanListFile {
        let mut ban_list = if config.fs.ban_list_file != PathBuf::new() {
            tracing::info!("Loading ban list from file: {}", config.fs.ban_list_file.display());

            let ban_list_file = std::fs::read_to_string(&config.fs.ban_list_file).unwrap();
            match toml::from_str(&ban_list_file) {
                Ok(ban_list) => ban_list,
                Err(e) => {
                    panic!("Failed to parse ban list file: {}", e);
                }
            }
        } else {
            BanListFile::default()
        };

        ban_list.subnets.extend(hardcoded_ban_list());

        ban_list
    }

    #[instrument(skip(self), level = "info")]
    pub(crate) fn ip_ban_list(&self) -> IpBanList {
        let mut ban_list = IpBanList::default();

        for subnet in &self.subnets {
            tracing::info!("Adding subnet to ban list: {}", subnet);
        }

        ban_list.banned_subnets.extend(&self.subnets);

        for ip in self.ips.clone() {
            tracing::info!("Adding ip to ban list: {}", ip);
            ban_list
                .generic_ban_list
                .ban(SocketAddr::new(ip, 0).ban_id(), Duration::from_secs(3600 * 24 * 365));
        }

        ban_list
    }
}

fn serialize_as_string<S>(value: &HashSet<IpSubnet>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(value.len()))?;
    for subnet in value {
        seq.serialize_element(&subnet.to_string())?;
    }

    seq.end()
}

fn deserialize_from_string<'de, D>(deserializer: D) -> Result<HashSet<IpSubnet>, D::Error>
where
    D: Deserializer<'de>,
{
    let string_value = Vec::<String>::deserialize(deserializer)?;

    string_value
        .into_iter()
        .map(|s| IpSubnet::from_str(&s))
        .collect::<Result<_, _>>()
        .map_err(serde::de::Error::custom)
}

fn hardcoded_ban_list() -> Vec<IpSubnet> {
    let subnets = [
        "91.198.115.0/24",
        "100.42.27.0/24",
        "162.218.65.0/24",
        "193.142.4.0/24",
        "199.116.84.0/24",
        "209.222.252.0/24",
        "23.92.36.0/26",
    ];

    subnets
        .into_iter()
        .map(IpSubnet::from_str)
        .collect::<Result<_, _>>()
        .unwrap()
}
