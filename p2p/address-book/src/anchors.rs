use crate::sealed::BorshNetworkZone;
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::DateTime;
use cuprate_helper::time::current_unix_timestamp;
use cuprate_p2p_core::{services::ZoneSpecificPeerListEntryBase, NetZoneAddress, NetworkZone};
use rand::prelude::*;
use std::task::{Context, Poll};
use std::time::Duration;
use std::{collections::HashMap, time::Instant};
use tokio_util::time::{delay_queue, DelayQueue};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AnchorPeer<A: NetZoneAddress> {
    pub peer: ZoneSpecificPeerListEntryBase<A>,
    pub anchor_until_timestamp: u64,
}

pub(crate) struct AnchorList<Z: BorshNetworkZone> {
    anchors: HashMap<Z::Addr, AnchorPeer<Z::Addr>>,
    anchor_timeouts: DelayQueue<Z::Addr>,
    anchor_timeout_keys: HashMap<Z::Addr, delay_queue::Key>,
}

impl<Z: BorshNetworkZone> AnchorList<Z> {
    pub(crate) fn new(anchors: Vec<AnchorPeer<Z::Addr>>) -> Self {
        let mut anchor_map = HashMap::new();
        let mut anchor_timeouts = DelayQueue::new();
        let mut anchor_timeout_keys = HashMap::new();
        for anchor in anchors {
            let date = DateTime::from_timestamp(anchor.anchor_until_timestamp as i64, 0).unwrap();

            tracing::info!("Got anchor peer: {}, set until: {}", anchor.peer.adr, date);

            let timeout_key = anchor_timeouts.insert(
                anchor.peer.adr.clone(),
                Duration::from_secs(
                    anchor
                        .anchor_until_timestamp
                        .saturating_sub(current_unix_timestamp()),
                ),
            );
            anchor_timeout_keys.insert(anchor.peer.adr.clone(), timeout_key);
            anchor_map.insert(anchor.peer.adr.clone(), anchor);
        }

        Self {
            anchors: anchor_map,
            anchor_timeouts,
            anchor_timeout_keys,
        }
    }

    pub(crate) fn poll_timeouts(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(exp)) = self.anchor_timeouts.poll_expired(cx) {
            tracing::info!("Removing anchor peer: {}, expired", exp.get_ref());

            self.anchors.remove(exp.get_ref());
            self.anchor_timeout_keys.remove(exp.get_ref());
        }
    }

    pub(crate) fn remove(
        &mut self,
        addr: &Z::Addr,
    ) -> Option<ZoneSpecificPeerListEntryBase<Z::Addr>> {
        tracing::info!("Removing anchor peer: {}", addr);

        let Some(anchor) = self.anchors.remove(addr) else {
            return None;
        };

        if let Some(timeout_key) = self.anchor_timeout_keys.remove(addr) {
            self.anchor_timeouts.remove(&timeout_key);
        }

        Some(anchor.peer)
    }

    pub(crate) fn len(&self) -> usize {
        self.anchors.len()
    }

    pub(crate) fn add(&mut self, peer: ZoneSpecificPeerListEntryBase<Z::Addr>) {
        let anchor_until_timestamp = get_anchor_lifetime();
        let date = DateTime::from_timestamp(anchor_until_timestamp as i64, 0).unwrap();

        tracing::info!(
            "Adding new anchor peer: {}, set until: {}",
            peer.adr,
            date.to_string()
        );

        let timeout_key = self.anchor_timeouts.insert(
            peer.adr.clone(),
            Duration::from_secs(anchor_until_timestamp.saturating_sub(current_unix_timestamp())),
        );
        self.anchor_timeout_keys
            .insert(peer.adr.clone(), timeout_key);
        self.anchors.insert(
            peer.adr.clone(),
            AnchorPeer {
                peer,
                anchor_until_timestamp,
            },
        );
    }

    pub(crate) fn anchors(&self) -> &HashMap<Z::Addr, AnchorPeer<Z::Addr>> {
        &self.anchors
    }

    pub(crate) fn contains(&self, addr: &Z::Addr) -> bool {
        self.anchors.contains_key(addr)
    }
}

fn get_anchor_lifetime() -> u64 {
    const MIN_ANCHOR_LIFETIME: u64 = 60 * 60 * 24 * 180;

    let dist = rand::distributions::Uniform::new(0, MIN_ANCHOR_LIFETIME);

    let extra = dist.sample(&mut thread_rng());

    current_unix_timestamp() + MIN_ANCHOR_LIFETIME + extra
}
