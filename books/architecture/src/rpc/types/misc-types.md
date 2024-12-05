# Misc types
Other than the main request/response types, this crate is also responsible
for any [miscellaneous types](https://doc.cuprate.org/cuprate_rpc_types/misc) used within `monerod`'s RPC.

For example, the `status` field within many RPC responses is defined within
[`cuprate-rpc-types`](https://doc.cuprate.org/cuprate_rpc_types/misc/enum.Status.html).

Types that aren't requests/responses but exist _within_ request/response
types are also defined in this crate, such as the
[`Distribution`](https://doc.cuprate.org/cuprate_rpc_types/misc/enum.Distribution.html)
structure returned from the [`get_output_distribution`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_output_distribution) method.