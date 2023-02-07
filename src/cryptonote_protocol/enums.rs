/// Methods tracking how a tx was received and relayed
pub enum RelayMethod {
        none,                           //< Received via RPC with `do_not_relay` set
        local,                            //< Received via RPC; trying to send over i2p/tor, etc.
        forward,                     //< Received over i2p/tor; timer delayed before ipv4/6 public broadcast
        stem,                           //< Received/send over network using Dandelion++ stem
        fluff,                             //< Received/sent over network using Dandelion++ fluff
        block,                           //< Received in block, takes precedence over others
}