use std::{sync::Arc, time::Duration};

use tokio::sync::Semaphore;

use monero_p2p::handles::HandleBuilder;

#[test]
fn send_ban_signal() {
    let (guard, mut connection_handle) = HandleBuilder::default().build();

    connection_handle.ban_peer(Duration::from_secs(300));

    let Some(ban_time) = connection_handle.check_should_ban() else {
        panic!("ban signal not received!");
    };

    assert_eq!(ban_time.0, Duration::from_secs(300));

    connection_handle.send_close_signal();
    assert!(guard.should_shutdown());

    guard.connection_closed();
    assert!(connection_handle.is_closed());
}

#[test]
fn multiple_ban_signals() {
    let (guard, mut connection_handle) = HandleBuilder::default().build();

    connection_handle.ban_peer(Duration::from_secs(300));
    connection_handle.ban_peer(Duration::from_secs(301));
    connection_handle.ban_peer(Duration::from_secs(302));

    let Some(ban_time) = connection_handle.check_should_ban() else {
        panic!("ban signal not received!");
    };

    // only the first will be seen
    assert_eq!(ban_time.0, Duration::from_secs(300));

    connection_handle.send_close_signal();
    assert!(guard.should_shutdown());

    guard.connection_closed();
    assert!(connection_handle.is_closed());
}

#[test]
fn dropped_guard_sends_disconnect_signal() {
    let semaphore = Arc::new(Semaphore::new(5));
    let (guard, connection_handle) = HandleBuilder::default()
        .with_permit(Some(semaphore.try_acquire_owned().unwrap()))
        .build();

    assert!(!connection_handle.is_closed());
    drop(guard);
    assert!(connection_handle.is_closed());
}
