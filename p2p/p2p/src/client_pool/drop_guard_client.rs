use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use cuprate_p2p_core::{client::Client, NetworkZone};

use crate::client_pool::ClientPool;

/// A wrapper around [`Client`] which returns the client to the [`ClientPool`] when dropped.
pub struct ClientPoolDropGuard<N: NetworkZone> {
    /// The [`ClientPool`] to return the peer to.
    pub(super) pool: Arc<ClientPool<N>>,
    /// The [`Client`].
    ///
    /// This is set to [`Some`] when this guard is created, then
    /// [`take`](Option::take)n and returned to the pool when dropped.
    pub(super) client: Option<Client<N>>,
}

impl<N: NetworkZone> Deref for ClientPoolDropGuard<N> {
    type Target = Client<N>;

    fn deref(&self) -> &Self::Target {
        self.client.as_ref().unwrap()
    }
}

impl<N: NetworkZone> DerefMut for ClientPoolDropGuard<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.client.as_mut().unwrap()
    }
}

impl<N: NetworkZone> Drop for ClientPoolDropGuard<N> {
    fn drop(&mut self) {
        let client = self.client.take().unwrap();

        self.pool.add_client(client);
    }
}
