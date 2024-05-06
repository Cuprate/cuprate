use monero_p2p::client::Client;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use monero_p2p::NetworkZone;

use crate::client_pool::ClientPool;

pub struct ClientPoolDropGuard<N: NetworkZone> {
    pub(super) pool: Arc<ClientPool<N>>,
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
