//! `crate::service` tests.
//!
//! This module contains general tests for the `service` implementation.
//!
//! Testing a thread-pool is slightly more complicated,
//! so this file provides TODO.

// This is only imported on `#[cfg(test)]` in `mod.rs`.

#![allow(unused_mut, clippy::significant_drop_tightening)]

//---------------------------------------------------------------------------------------------------- Use
use tower::{Service, ServiceExt};

use cuprate_types::service::{ReadRequest, Response, WriteRequest};

use crate::{
    config::Config,
    service::{init, DatabaseReadHandle, DatabaseWriteHandle},
};

//---------------------------------------------------------------------------------------------------- Tests
/// Initialize the `service`.
fn init_service() -> (DatabaseReadHandle, DatabaseWriteHandle, tempfile::TempDir) {
    let tempdir = tempfile::tempdir().unwrap();
    let config = Config::low_power(Some(tempdir.path().into()));
    let (reader, writer) = init(config).unwrap();
    (reader, writer, tempdir)
}

/// Simply `init()` the service and then drop it.
///
/// If this test fails, something is very wrong.
#[test]
fn init_drop() {
    let (reader, writer, _tempdir) = init_service();
}

// /// Send a read request, and receive a response,
// /// asserting the response the expected value.
// #[tokio::test]
// async fn read_request() {
//     let (reader, writer, _tempdir) = init_service();

//     for (request, expected_response) in [
//         (ReadRequest::Example1, Response::Example1),
//         (ReadRequest::Example2(123), Response::Example2(123)),
//         (
//             ReadRequest::Example3("hello".into()),
//             Response::Example3("hello".into()),
//         ),
//     ] {
//         // This calls `poll_ready()` asserting we have a permit before `call()`.
//         let response_channel = reader.clone().oneshot(request);
//         let response = response_channel.await.unwrap();
//         assert_eq!(response, expected_response);
//     }
// }

// /// Send a write request, and receive a response,
// /// asserting the response the expected value.
// #[tokio::test]
// async fn write_request() {
//     let (reader, mut writer, _tempdir) = init_service();

//     for (request, expected_response) in [
//         (WriteRequest::Example1, Response::Example1),
//         (WriteRequest::Example2(123), Response::Example2(123)),
//         (
//             WriteRequest::Example3("hello".into()),
//             Response::Example3("hello".into()),
//         ),
//     ] {
//         let response_channel = writer.call(request);
//         let response = response_channel.await.unwrap();
//         assert_eq!(response, expected_response);
//     }
// }
