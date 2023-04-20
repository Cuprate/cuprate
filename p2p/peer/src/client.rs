pub struct Client {
    server_tx: mpsc::Sender<ClientRequest>,
}
