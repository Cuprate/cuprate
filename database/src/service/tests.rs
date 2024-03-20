//! `crate::service` tests.
//!
//! This module contains general tests for the `service` implementation.
//!
//! Testing a thread-pool is slightly more complicated,
//! so this file provides TODO.

// This is only imported on `#[cfg(test)]` in `mod.rs`.

#![allow(unused_mut)]

use tower::Service;

//---------------------------------------------------------------------------------------------------- Use
use crate::{
    config::Config,
    service::{init, DatabaseReadHandle, DatabaseWriteHandle, ReadRequest, Response, WriteRequest},
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

/// Send a read request, and receive a response,
/// asserting the response the expected value.
#[tokio::test]
async fn read_request() {
    let (mut reader, writer, _tempdir) = init_service();

    let request = ReadRequest::Example1;
    let response_channel = reader.call(request);
    let response = response_channel.await.unwrap();
    assert_eq!(response, Response::Example1);

    let request = ReadRequest::Example2(123);
    let response_channel = reader.call(request);
    let response = response_channel.await.unwrap();
    assert_eq!(response, Response::Example2(123));

    let request = ReadRequest::Example3("hello".into());
    let response_channel = reader.call(request);
    let response = response_channel.await.unwrap();
    assert_eq!(response, Response::Example3("hello".into()));
}

/// Send a write request, and receive a response,
/// asserting the response the expected value.
#[tokio::test]
async fn write_request() {
    let (reader, mut writer, _tempdir) = init_service();

    let request = WriteRequest::Example1;
    let response_channel = writer.call(request);
    let response = response_channel.await.unwrap();
    assert_eq!(response, Response::Example1);

    let request = WriteRequest::Example2(123);
    let response_channel = writer.call(request);
    let response = response_channel.await.unwrap();
    assert_eq!(response, Response::Example2(123));

    let request = WriteRequest::Example3("hello".into());
    let response_channel = writer.call(request);
    let response = response_channel.await.unwrap();
    assert_eq!(response, Response::Example3("hello".into()));
}
