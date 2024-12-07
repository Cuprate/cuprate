use futures::{FutureExt, StreamExt};

use cuprate_async_buffer::new_buffer;

#[tokio::test]
async fn async_buffer_send_rec() {
    let (mut tx, mut rx) = new_buffer(1000);

    tx.send(4, 5).await.unwrap();
    tx.send(8, 5).await.unwrap();

    assert_eq!(rx.next().await.unwrap(), 4);
    assert_eq!(rx.next().await.unwrap(), 8);
}

#[tokio::test]
async fn capacity_reached() {
    let (mut tx, mut rx) = new_buffer(1000);

    tx.send(4, 1000).await.unwrap();

    assert!(tx.ready(1).now_or_never().is_none());

    let fut = tx.ready(1);

    rx.next().await;

    assert!(fut.now_or_never().is_some());
}

#[tokio::test]
async fn single_item_over_capacity() {
    let (mut tx, mut rx) = new_buffer(1000);
    tx.send(4, 1_000_000).await.unwrap();

    assert_eq!(rx.next().await.unwrap(), 4);
}
