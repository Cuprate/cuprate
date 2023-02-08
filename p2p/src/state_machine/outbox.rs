use std::collections::VecDeque;


use cuprate_net::{bucket::header::P2pCommand, NetworkAddress};

use super::DisconnectReason;

pub enum MsgType {
    Notification,
    Request,
    Response
}

pub enum InternalEvent {
    PeerNegotiated(NetworkAddress),
}

pub enum Event {
    Internal(InternalEvent),
    SendMsg(NetworkAddress, P2pCommand, MsgType),
    SetTimer(i64),
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

    fn send_msg(&mut self, addr: NetworkAddress, msg: P2pCommand, ty: MsgType) {
        self.buffer.push_back(Event::SendMsg(addr, msg, ty))
    }

    pub fn connect(&mut self, addr: NetworkAddress) {
        self.buffer.push_back(Event::Connect(addr))
    }

    pub fn disconnect(&mut self, addr: NetworkAddress, reason: DisconnectReason) {
        self.buffer.push_back(Event::Disconnect(addr, reason))
    }



    pub fn send_handshake_request(&mut self, addr: NetworkAddress) {
        self.send_msg(addr, P2pCommand::Handshake, MsgType::Request);
    }

    pub fn send_handshake_response(&mut self, addr: NetworkAddress) {
        self.send_msg(addr, P2pCommand::Handshake, MsgType::Response);
    }
}

impl Iterator for OutBox {
    type Item = Event;
    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.pop_front()
    }
}