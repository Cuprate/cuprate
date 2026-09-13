# Tor

`cuprated` is capable of connecting to the Tor network for interacting with anonymous peers or using it to anonymize your nodes on the internet.

## Transaction broadcasting

The primary intent of this feature, similarly to `monerod`, is to improve your privacy by hiding the origin of a transaction broadcasted on the network. Using an anonymous overlay network yields better privacy improvements than mitigations like Dandelion++.
When `cuprated`'s Tor mode is enabled, user-initiated transactions will be broadcasted to a peer over Tor, or will fail if `cuprated` could not select one.

## Connecting to the network

`cuprated` can connect to the Tor network in two different ways:

- **Arti**: `cuprated` comes with the `Arti` library embedded, which permits it to bootstrap an internal Tor router within the `cuprated` process. This router is then used to connect to or create hidden services.

- **Tor daemon**: `cuprated` will connect to a running Tor daemon address. It will listen for incoming connections from there and use a SOCKS5 interface to connect to hidden services. This is similar to how `monerod` actually connects to Tor through `tx-proxy` and `anonymous-inbound` flags.

> **🤷 What method to choose?**
>
> **Arti is recommended for beginners** because it is capable of generating a hidden service automatically. This makes accepting inbound connections much easier to configure. It also shows decent performance for usual usage.
>
> **Consider using an external Tor daemon for any advanced configurations**, such as vanity addresses, pluggable transports, and circuit geo-restrictions.

## Enabling Tor

First, select the mode you want to use in the `[tor]` section of the TOML configuration file:

```toml
## Configuration for cuprated's Tor component
[tor]
## Enable Tor network by specifying how to connect to it.
##
## When "Daemon" is set, the Tor daemon address to use can be
## specified in `tor.daemon.address`.
##
## Type         | String
## Valid values | "Arti", "Daemon", "Off"
## Examples     | "Arti"
mode = "Off" # <----- Here
```

Other sections of the configuration file will then become relevant:

| Section                             | Purpose                              | Notes                                                                  |
|-------------------------------------|--------------------------------------|------------------------------------------------------------------------|
| `[tor.arti]`                        | Arti's specific parameters           | Only relevant if `mode` = `"Arti"` or `p2p.clear_net.proxy` = `"Arti"`
| `[tor.daemon]`                      | Tor mode's specific parameters       | Only relevant if `mode` = `"Daemon"`
| `[p2p.tor_net]`                     | Tor P2P zone parameters              |
| `[p2p.tor_net.address_book_config]` | Tor P2P zone address book parameters | Avoid tuning this section if you do not know what you are doing.

### Arti

If you selected the `"Arti"` mode, you can start your node right away. `cuprated` will take care of bootstrapping to the Tor network at boot and connecting to Tor peers.

### Tor Daemon

If you selected the `"Daemon"` mode, a field within the `[tor.daemon]` section requires your attention.

The `address` field signifies the SOCKS5 address of a/your Tor daemon that `cuprated` can use to initiate connections towards Tor. These are for outgoing connections. Most of the time, the system Tor daemon opens this interface on port 9050, in which case the default should be left.

## Accepting inbound connections

In order to participate in propagating transactions of other peers, your node must accept inbound connections over Tor. This requires the use of a hidden service, or commonly known as an onion service.

To start, enable the `inbound_onion` option in your TOML configuration file:

```toml
[p2p.tor_net]
#...
## Enable Tor inbound onion server.
##
## [...]
##
## Type         | boolean
## Valid values | false, true
## Examples     | false
inbound_onion = true
```

### Arti

If you are using Arti, that's it. `cuprated` will auto-generate a hidden service at startup.

A few notes:
- The onion address is generated randomly.
- The onion address is persistent across reboots.
- The onion address cannot be changed without deleting the Arti state directory.

### Daemon

If you are using Daemon, three fields need to be edited.

You first need to create a hidden service within your Tor daemon configuration file. If you do not know how, please follow this guide: https://community.torproject.org/onion-services/setup/ while ignoring steps 1 and 6.

Assuming that your hidden service is operational, that your torrc looks like this:
```
HiddenServiceDir /var/lib/tor/my_awesome_cuprated_hs/
HiddenServicePort 18083 127.0.0.1:18090
```
and supposing that your onion address is `allyouhavetodecideiswhattodowiththetimethatisgiventoyouu.onion`.

In the `[tor.daemon]` section,
The `listening_addr` field must be set to the IP and port on which `cuprated` will listen for connections coming from your Tor daemon. This is the destination of your hidden service.
```
HiddenServicePort 18083 [127.0.0.1:18090] <-- This part
```
The `anonymous_inbound` field must be set to the onion address of your hidden service. In this case, `"allyouhavetodecideiswhattodowiththetimethatisgiventoyouu.onion"`.

In the `[p2p.tor_net]` section,
The `p2p_port` field must be set to the port that other Tor peers will attempt to establish connections to. This is the virtual port of your hidden service.
```
HiddenServicePort [18083] 127.0.0.1:18090
                     ^
              The virtual port
```

If everything has been set correctly, your node will start to broadcast its onion address to other Tor peers in order to be reached.

## Anonymizing cuprate

`cuprated` can use Arti to anonymize internet connections to other nodes. This makes it possible to sync with the rest of the network while your node remains anonymous within the bounds of the Tor network.

To enable this mode, set the `p2p.clear_net.proxy` configuration field to `"Arti"`.

> **⚠️ Warning ⚠️**
>
> A few caveats must be acknowledged:
>
> - The blockchain is of significant size for low-throughput networks such as Tor. You should expect the syncing process to be much longer.
> - Connecting to a lot of peers will put a load on your Tor circuit's nodes, which can eventually evict you. You should be conservative with your maximum number of peers.
> - Incoming internet connections are disabled in this mode. This is inherent to the Monero P2P protocol, which identifies the remote IP address as being end-to-end routable. However, your node will be seen through Tor exit nodes, which refuse incoming connections and are even more unlikely to forward traffic to you specifically.
