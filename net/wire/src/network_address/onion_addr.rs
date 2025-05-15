//! Onion address
//!
//! This module define v3 tor onion addresses
//!
//!

use std::{fmt::Display, str::FromStr};

use thiserror::Error;

use super::{NetworkAddress, NetworkAddressIncorrectZone};

/// A v3, `Copy`able onion address.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub struct OnionAddr {
    // 56 characters encoded onion v3 domain without the .onion suffix
    // <https://spec.torproject.org/rend-spec/encoding-onion-addresses.html>
    pub domain: [u8; 56],
    // Virtual port of the peer
    pub port: u16,
}

/// Error enum at parsing onion addresses
#[derive(Debug, Error)]
pub enum OnionAddrParsingError {
    #[error("Address is either too long or short, length: {0}")]
    InvalidLength(usize),
    #[error("Address contain non-utf8 code point at tld byte location: {0:x}")]
    NonUtf8Char(u8),
    #[error("This is not an onion address, Tld: {0}")]
    InvalidTld(String),
    #[error("Domain contains non base32 characters")]
    NonBase32Char,
    #[error("Invalid port specified")]
    InvalidPort,
}

impl OnionAddr {
    /// Attempt to create an [`OnionAddr`] from a complete .onion address string and a port.
    ///
    /// Return None if the supplied `addr` is invalid.
    pub fn new(addr: &str, port: u16) -> Option<Self> {
        Self::check_addr(addr).is_ok().then_some(Self {
            domain: addr.as_bytes()[..56].try_into().ok()?,
            port,
        })
    }

    /// Establish if the .onion address is valid.
    ///
    /// Return `true` if valid, `false` otherwise.
    pub fn check_addr(addr: &str) -> Result<(), OnionAddrParsingError> {
        // v3 onion addresses are 62 characters long
        if addr.len() != 62 {
            return Err(OnionAddrParsingError::InvalidLength(addr.len()));
        }

        let Some((domain, tld)) = addr.split_at_checked(56) else {
            return Err(OnionAddrParsingError::NonUtf8Char(addr.as_bytes()[56]));
        };

        // The ".onion" suffix must be located at the 57th byte.
        if tld != ".onion" {
            return Err(OnionAddrParsingError::InvalidTld(String::from(tld)));
        }

        // The domain part must only contain base32 characters.
        if !domain
            .as_bytes()
            .iter()
            .copied()
            .all(|c| c.is_ascii_lowercase() || (b'2'..=b'7').contains(&c))
        {
            return Err(OnionAddrParsingError::NonBase32Char);
        }

        Ok(())
    }

    pub const fn port(&self) -> u16 {
        self.port
    }
}

/// Display for [`OnionAddr`] only print the **onion address**. Not the port.
impl Display for OnionAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = self
            .domain
            .utf8_chunks()
            .next()
            .expect("Onion addresses are always containing UTF-8 characters.");

        f.write_str(str.valid())?;
        f.write_str(".onion")
    }
}

/// [`OnionAddr`] parses an onion address **and a port**.
impl FromStr for OnionAddr {
    type Err = OnionAddrParsingError;

    fn from_str(addr: &str) -> Result<Self, Self::Err> {
        let (addr, port) = addr
            .split_at_checked(62)
            .ok_or(OnionAddrParsingError::InvalidLength(addr.len()))?;

        // Port
        let port: u16 = port
            .starts_with(':')
            .then_some(port[1..].parse().ok())
            .flatten()
            .ok_or(OnionAddrParsingError::InvalidPort)?;

        // Address
        Self::check_addr(addr)?;
        let domain = addr.as_bytes()[..56]
            .try_into()
            .unwrap_or_else(|e| panic!("We just validated address: {addr} : {e}"));

        Ok(Self { domain, port })
    }
}

impl TryFrom<NetworkAddress> for OnionAddr {
    type Error = NetworkAddressIncorrectZone;
    fn try_from(value: NetworkAddress) -> Result<Self, Self::Error> {
        match value {
            NetworkAddress::Tor(addr) => Ok(addr),
            NetworkAddress::Clear(_) => Err(NetworkAddressIncorrectZone),
        }
    }
}

impl From<OnionAddr> for NetworkAddress {
    fn from(value: OnionAddr) -> Self {
        Self::Tor(value)
    }
}

#[cfg(test)]
mod tests {
    use super::OnionAddr;

    const VALID_ONION_ADDRESSES: &[&str] = &[
        "2gzyxa5ihm7nsggfxnu52rck2vv4rvmdlkiu3zzui5du4xyclen53wid.onion", // Tor Website
        "pzhdfe7jraknpj2qgu5cz2u3i4deuyfwmonvzu5i3nyw4t4bmg7o5pad.onion", // Tor Blog
        "monerotoruzizulg5ttgat2emf4d6fbmiea25detrmmy7erypseyteyd.onion", // Monero Website
        "sfprivg7qec6tdle7u6hdepzjibin6fn3ivm6qlwytr235rh5vc6bfqd.onion", // SethForPrivacy
        "yucmgsbw7nknw7oi3bkuwudvc657g2xcqahhbjyewazusyytapqo4xid.onion", // P2Pool
        "d6ac5qatnyodxisdehb3i4m7edfvtukxzhhtyadbgaxghcxee2xadpid.onion", // Rucknium ♥
        "allyouhavetodecideiswhattodowiththetimethatisgiventoyouu.onion", // Gandalf the Grey
    ];

    const INVALID_ONION_ADDRESSES: &[&str] = &[
        "expyuzz4wqqyqhjn.onion",                                         // V2 onion address (too short)
        "anamountoflengthytextthathasbeenrepeatedlycopiedfromsomwhereandpastedasareplytoanirrelevantsubject.onion", // Too large (Copypasta definition)
        "XOe4vN5uwdztif6GOAZfbmogH6wh5jc4up35bqdFLU6bkdc5cas5vjqd.onion", // Uppercases (PrivacyGuides.org)
        "revuo00joezkbeitqmas8ab9spbrkr9vzbhjmeuv01ovrfqfp11mtjid.onion", // Wrong numbers (Revuo)
        "featherdvtpi7ckdbkb2yxjfwx3oyvr3xjz3oo4rszylfzjdg6pbm3id.wallt", // Wrong TLD (Feather wallet)
        "p2pool.onion2giz2r5cpqicajwoazjcxkfujxswtk3jolfk2ubilhrkqam2id", // Displaced TLD (P2Pool Observer)
        "duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczadnion"  // Not UTF-8 at 56th byte (DuckDuckGo)
    ];

    const VALID_ONION_ADDRESSES_WITH_PORT: &[&str] = &[
        // Tor mainnet seed nodes as of 2025-05-15 with random ports
        "zbjkbsxc5munw3qusl7j2hpcmikhqocdf4pqhnhtpzw5nt5jrmofptid.onion:65535",
        "qz43zul2x56jexzoqgkx2trzwcfnr6l3hbtfcfx54g4r3eahy3bssjyd.onion:17426",
        "plowsof3t5hogddwabaeiyrno25efmzfxyro2vligremt7sxpsclfaid.onion:0",
        "plowsoffjexmxalw73tkjmf422gq6575fc7vicuu4javzn2ynnte6tyd.onion:18083",
        "plowsofe6cleftfmk2raiw5h2x66atrik3nja4bfd3zrfa2hdlgworad.onion:1398",
        "aclc4e2jhhtr44guufbnwk5bzwhaecinax4yip4wr4tjn27sjsfg6zqd.onion:14691",
    ];

    const INVALID_ONION_ADDRESSES_WITH_PORT: &[&str] = &[
        "zbjkbsxc5munw3qusl7j2hpcmikhqocdf4pqhnhtpzw5nt5jrmofptid.onion:65536", // Too high
        "qz43zul2x56jexzoqgkx2trzwcfnr6l3hbtfcfx54g4r3eahy3bssjyd.onion:-17426", // Negative
        "plowsof3t5hogddwabaeiyrno25efmzfxyro2vligremt7sxpsclfaid.onion::::0",  // starts with :
        "plowsof3t5hogddwabaeiyrno25efmzfxyro2vligremt7sxpsclfaid.onionN0",     // starts with N
        "plowsoffjexmxalw73tkjmf422gq6575fc7vicuu4javzn2ynnte6tyd.onion:18083more", // letters
        "plowsofe6cleftfmk2raiw5h2x66atrik3nja4bfd3zrfa2hdlgworad.onion:",  // Not UTF-8
        "aclc4e2jhhtr44guufbnwk5bzwhaecinax4yip4wr4tjn27sjsfg6zqd.onion:",      // Empty
    ];

    #[test]
    fn valid_onion_address() {
        for addr in VALID_ONION_ADDRESSES {
            assert!(
                OnionAddr::check_addr(addr).is_ok(),
                "Address {addr} has been reported as invalid."
            );
        }
    }

    #[test]
    fn invalid_onion_address() {
        for addr in INVALID_ONION_ADDRESSES {
            assert!(
                OnionAddr::check_addr(addr).is_err(),
                "Address {addr} has been reported as valid."
            );
        }
    }

    #[test]
    fn parse_valid_onion_address_w_port() {
        for addr in VALID_ONION_ADDRESSES_WITH_PORT {
            assert!(
                addr.parse::<OnionAddr>().is_ok(),
                "Address {addr} has been reported as invalid."
            );
        }
    }

    #[test]
    fn parse_invalid_onion_address_w_port() {
        for addr in INVALID_ONION_ADDRESSES_WITH_PORT {
            assert!(
                addr.parse::<OnionAddr>().is_err(),
                "Address {addr} has been reported as valid."
            );
        }
    }
}
