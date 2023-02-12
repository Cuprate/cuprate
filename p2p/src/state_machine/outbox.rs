use std::collections::VecDeque;


use cuprate_net::{P2pCommand, NetworkAddress};

use super::DisconnectReason;

#[derive(Debug)]
pub enum InternalEvent {
    PeerNegotiated(NetworkAddress),
}

#[derive(Debug)]
pub enum Event {
    Internal(InternalEvent),
    SendReq(NetworkAddress, P2pCommand),
    SendRes(NetworkAddress, P2pCommand),
    SendNoti(NetworkAddress, P2pCommand),
    SetTimer(u64),
    Disconnect(NetworkAddress, DisconnectReason),
    Connect(NetworkAddress)
}

pub struct OutBox {
    buffer: VecDeque<Event>,
}

impl OutBox {
    pub fn new() -> OutBox {
        OutBox { buffer: VecDeque::new() }
    }

    fn send_req(&mut self, addr: NetworkAddress, msg: P2pCommand) {
        self.buffer.push_back(Event::SendReq(addr, msg))
    }

    fn send_res(&mut self, addr: NetworkAddress, msg: P2pCommand) {
        self.buffer.push_back(Event::SendRes(addr, msg))
    }

    pub fn connect(&mut self, addr: NetworkAddress) {
        self.buffer.push_back(Event::Connect(addr))
    }

    pub fn disconnect(&mut self, addr: NetworkAddress, reason: DisconnectReason) {
        self.buffer.push_back(Event::Disconnect(addr, reason))
    }



    pub fn send_handshake_request(&mut self, addr: NetworkAddress) {
        self.send_req(addr, P2pCommand::Handshake);
    }

    pub fn send_handshake_response(&mut self, addr: NetworkAddress) {
        self.send_res(addr, P2pCommand::Handshake);
    }
}

impl Iterator for OutBox {
    type Item = Event;
    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.pop_front()
    }
}