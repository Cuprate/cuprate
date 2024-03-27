use std::ops::{Deref, DerefMut};

use tokio::sync::mpsc;

use monero_p2p::{client::PeakEwmaClient, NetworkZone};

/// A [`PeakEwmaClient`] that returns itself to the peer set when dropped.
pub struct DropGuardClient<N: NetworkZone> {
    /// The client.
    ///
    /// Must stay [`Some`] until drop.
    client: Option<PeakEwmaClient<N>>,
    /// The return channel.
    ret_channel: mpsc::Sender<PeakEwmaClient<N>>,
}

impl<N: NetworkZone> DropGuardClient<N> {
    pub fn new(client: PeakEwmaClient<N>, ret_channel: mpsc::Sender<PeakEwmaClient<N>>) -> Self {
        Self {
            client: Some(client),
            ret_channel,
        }
    }
}

impl<N: NetworkZone> Deref for DropGuardClient<N> {
    type Target = PeakEwmaClient<N>;

    fn deref(&self) -> &Self::Target {
        self.client.as_ref().unwrap()
    }
}

impl<N: NetworkZone> DerefMut for DropGuardClient<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.client.as_mut().unwrap()
    }
}

impl<N: NetworkZone> Drop for DropGuardClient<N> {
    fn drop(&mut self) {
        if self
            .ret_channel
            .try_send(self.client.take().unwrap())
            .is_err()
        {
            tracing::debug!("Error returning peer to peer set. Disconnecting.")
        }
    }
}
