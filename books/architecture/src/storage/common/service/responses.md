# Responses
After sending a request using the read/write handle, the value returned is _not_ the response, yet an `async`hronous channel that will eventually return the response:
```rust,ignore
// Send a request.
//                                   tower::Service::call()
//                                          V
let response_channel: Channel = read_handle.call(BlockchainReadRequest::ChainHeight)?;

// Await the response.
let response: BlockchainReadRequest = response_channel.await?;
```

After `await`ing the returned channel, a `Response` will eventually be returned when
the `Service` threadpool has fetched the value from the database and sent it off.

Both read/write requests variants match in name with `Response` variants, i.e.
- `BlockchainReadRequest::ChainHeight` leads to `BlockchainResponse::ChainHeight`
- `BlockchainWriteRequest::WriteBlock` leads to `BlockchainResponse::WriteBlockOk`
