use std::collections::HashMap;

use crate::{header::Flags, Direction};

use super::{BodyDeEnc, Bucket, BucketHead, BucketStream, ISMOutput, InternalStateMachine};

#[derive(Debug)]
pub enum Output<A, R> {
    Connect(A),
    Disconnect(A, R),
    Write(A, Vec<u8>),
    SetTimer(std::time::Duration),
}

pub struct Levin<S: InternalStateMachine> {
    peer_buffer: HashMap<S::PeerID, BucketStream>,
    internal_sm: S,
}

impl<S: InternalStateMachine> Levin<S> {
    pub fn new(internal_sm: S) -> Self {
        Levin { 
            peer_buffer: HashMap::new(), 
            internal_sm 
        }
    }

    pub fn tick(&mut self) {
        self.internal_sm.tick();
    }

    pub fn wake(&mut self) {
        self.internal_sm.wake();
    }

    pub fn connected(&mut self, addr: S::PeerID, direction: Direction) {
        self.peer_buffer.insert(addr.clone(), BucketStream::default());
        self.internal_sm.connected(&addr, direction);
    }

    pub fn disconnected(&mut self, addr: S::PeerID) {
        self.peer_buffer.remove(&addr);
        self.internal_sm.disconnected(&addr);
    }

    fn handle_bucket(&mut self, addr: &S::PeerID, bucket: Bucket) {
        // Request
        if bucket.header.flags == Flags::REQUEST && bucket.header.have_to_return_data {
            let body = S::BodyRequest::decode_body(&bucket.body, bucket.header.command);
            match body {
                Ok(body) => self.internal_sm.received_request(addr, body),
                Err(e) => self.internal_sm.error_decoding_bucket(e),
            };
        }
        // Response
        else if bucket.header.flags == Flags::RESPONSE && !bucket.header.have_to_return_data {
            // levin checks if return_code > 0 if it's a response which we should do here.
            let body = S::BodyResponse::decode_body(&bucket.body, bucket.header.command);
            match body {
                Ok(body) => self.internal_sm.received_response(addr, body),
                Err(e) => self.internal_sm.error_decoding_bucket(e),
            };
        }
        // Notification
        else if bucket.header.flags == Flags::REQUEST && !bucket.header.have_to_return_data {
            let body = S::BodyNotification::decode_body(&bucket.body, bucket.header.command);
            match body {
                Ok(body) => self.internal_sm.received_notification(addr, body),
                Err(e) => self.internal_sm.error_decoding_bucket(e),
            };
        }
    }

    pub fn received_bytes(&mut self, addr: &S::PeerID, buf: &[u8]) {
        let stream = self.peer_buffer.get_mut(addr);
        let Some(stream) = stream else {
            // Peer sent bytes but isn't connected??
            return;
        };
        stream.received_bytes(buf);

        let mut err = false;

        let mut buckets = Vec::new();

        while let Some(bucket) = stream.decode_next_bucket().unwrap_or_else(|e| {
            self.internal_sm.error_decoding_bucket(e);
            err = true;
            None
        }) {
            buckets.push(bucket);
        }

        if err {
            return;
        }
        for bucket in buckets {
            self.handle_bucket(addr, bucket)
        }
    }
}

impl<S: InternalStateMachine> Iterator for Levin<S> {
    type Item = Output<S::PeerID, S::DisconnectReason>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.internal_sm.next()? {
            ISMOutput::Connect(addr) => Output::Connect(addr),
            ISMOutput::Disconnect(addr, r) => Output::Disconnect(addr, r),
            ISMOutput::SetTimer(time) => Output::SetTimer(time),
            ISMOutput::WriteNotification(addr, noti) => {
                Output::Write(addr, self.build_notification_bucket(noti).to_bytes())
            }
            ISMOutput::WriteResponse(addr, res) => {
                Output::Write(addr, self.build_response_bucket(res).to_bytes())
            }
            ISMOutput::WriteRequest(addr, req) => {
                Output::Write(addr, self.build_request_bucket(req).to_bytes())
            }
        })
    }
}

impl<S: InternalStateMachine> Levin<S> {
    fn build_notification_bucket(&self, noti: S::BodyNotification) -> Bucket {
        let (body, command) = noti.encode_body();

        let header = BucketHead::build(body.len() as u64, false, command, Flags::REQUEST, 0);

        Bucket { header, body }
    }

    fn build_response_bucket(&self, res: S::BodyResponse) -> Bucket {
        let (body, command) = res.encode_body();

        let header = BucketHead::build(body.len() as u64, false, command, Flags::RESPONSE, 1);

        Bucket { header, body }
    }

    fn build_request_bucket(&self, req: S::BodyRequest) -> Bucket {
        let (body, command) = req.encode_body();

        let header = BucketHead::build(body.len() as u64, true, command, Flags::REQUEST, 0);

        Bucket { header, body }
    }
}
