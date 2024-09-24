//! Functions for TODO: doc enum message.

use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
};

use anyhow::{anyhow, Error};
use futures::StreamExt;
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_p2p_core::{
    client::handshaker::builder::DummyAddressBook,
    services::{AddressBookRequest, AddressBookResponse},
    AddressBook, ClearNet,
};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainWriteRequest},
    Chain, ExtendedBlockHeader, HardFork, OutputOnChain, VerifiedBlockInformation,
};

use crate::rpc::{CupratedRpcHandler, CupratedRpcHandlerState};

#[expect(clippy::needless_pass_by_ref_mut, reason = "TODO: remove after impl")]
impl CupratedRpcHandlerState {
    /// [`AddressBookRequest::PeerlistSize`]
    pub(super) async fn peerlist_size(&mut self) -> Result<(u64, u64), Error> {
        let AddressBookResponse::<ClearNet>::PeerlistSize { white, grey } =
            <DummyAddressBook as tower::ServiceExt<AddressBookRequest<ClearNet>>>::ready(
                &mut DummyAddressBook,
            )
            .await
            .expect("TODO")
            .call(AddressBookRequest::PeerlistSize)
            .await
            .expect("TODO")
        else {
            unreachable!();
        };

        Ok((usize_to_u64(white), usize_to_u64(grey)))
    }

    /// [`AddressBookRequest::ConnectionCount`]
    pub(super) async fn connection_count(&mut self) -> Result<(u64, u64), Error> {
        let AddressBookResponse::<ClearNet>::ConnectionCount { incoming, outgoing } =
            <DummyAddressBook as tower::ServiceExt<AddressBookRequest<ClearNet>>>::ready(
                &mut DummyAddressBook,
            )
            .await
            .expect("TODO")
            .call(AddressBookRequest::ConnectionCount)
            .await
            .expect("TODO")
        else {
            unreachable!();
        };

        Ok((usize_to_u64(incoming), usize_to_u64(outgoing)))
    }

    /// [`AddressBookRequest::SetBan`]
    pub(super) async fn set_ban(&mut self, peer: Infallible) -> Result<(), Error> {
        let AddressBookResponse::<ClearNet>::Ok = <DummyAddressBook as tower::ServiceExt<
            AddressBookRequest<ClearNet>,
        >>::ready(&mut DummyAddressBook)
        .await
        .expect("TODO")
        .call(AddressBookRequest::SetBan(peer))
        .await
        .expect("TODO") else {
            unreachable!();
        };

        Ok(())
    }

    /// [`AddressBookRequest::GetBan`]
    pub(super) async fn get_ban(&mut self, peer: Infallible) -> Result<(), Error> {
        let AddressBookResponse::<ClearNet>::GetBan(ban) =
            <DummyAddressBook as tower::ServiceExt<AddressBookRequest<ClearNet>>>::ready(
                &mut DummyAddressBook,
            )
            .await
            .expect("TODO")
            .call(AddressBookRequest::GetBan(peer))
            .await
            .expect("TODO")
        else {
            unreachable!();
        };

        Ok(())
    }

    /// [`AddressBookRequest::GetBans`]
    pub(super) async fn get_bans(&mut self) -> Result<(), Error> {
        let AddressBookResponse::<ClearNet>::GetBans(bans) =
            <DummyAddressBook as tower::ServiceExt<AddressBookRequest<ClearNet>>>::ready(
                &mut DummyAddressBook,
            )
            .await
            .expect("TODO")
            .call(AddressBookRequest::GetBans)
            .await
            .expect("TODO")
        else {
            unreachable!();
        };

        Ok(todo!())
    }
}
