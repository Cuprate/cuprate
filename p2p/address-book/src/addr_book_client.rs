use std::pin::Pin;
use std::future::Future;

use futures::{SinkExt, FutureExt};
use futures::channel::{mpsc, oneshot};
use monero_wire::network_address::NetZone;

use crate::address_book::{AddressBookRequest, AddressBookResponse, AddressBookClientRequest};
use crate::AddressBookError;

pub struct AddressBookClient {
    public: Option<mpsc::Sender<AddressBookClientRequest>>,
    tor: Option<mpsc::Sender<AddressBookClientRequest>>,
    i2p: Option<mpsc::Sender<AddressBookClientRequest>>,
}

async fn send_req_to_chan(chan: Option<mpsc::Sender<AddressBookClientRequest>>, req: AddressBookClientRequest) -> Result<(), AddressBookError> {
    if let Some(mut chan) = chan {
        chan.send(req).await.map_err(|_| AddressBookError::AddressBooksChannelClosed)
    } else {
        unreachable!("If we are getting requests to this addr book the book should have been started")
    }
}

impl AddressBookClient {
    fn get_chan_to_route(&mut self, zone: NetZone) -> Option<mpsc::Sender<AddressBookClientRequest>> {
        match zone {
            NetZone::Public => self.public.clone(),
            NetZone::Tor => self.tor.clone(),
            NetZone::I2p => self.i2p.clone(),
        }

    }
}


impl tower::Service<AddressBookRequest> for AddressBookClient {
    type Error = AddressBookError;
    type Response = AddressBookResponse;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        let mut all_ready = true;
    
        let mut check_channel = |chan: &mut Option<mpsc::Sender<AddressBookClientRequest>> | -> Result<(), Self::Error> {
            if let Some(chan) = chan.as_mut() {
                match chan.poll_ready(cx) {
                    std::task::Poll::Ready(Ok(_)) => {}
                    std::task::Poll::Ready(Err(_)) => return Err(AddressBookError::AddressBooksChannelClosed),
                    std::task::Poll::Pending => all_ready = false,
                }
            }
            Ok(())
        };
    
    
        check_channel(&mut self.public)?;
    
        check_channel(&mut self.tor)?;

        check_channel(&mut self.i2p)?;
    
        if all_ready {
            std::task::Poll::Ready(Ok(()))
        } else {
            std::task::Poll::Pending
        }
    }

    fn call(&mut self, req: AddressBookRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();

        let zone = req.get_zone();

        let req = AddressBookClientRequest {
            req,
            tx
        };

        let chan = self.get_chan_to_route(zone);


        async move {
            send_req_to_chan(chan, req).await?;

            rx.await.expect("Address Book will not drop requests until completed")
        }.boxed()
    }
        
}
