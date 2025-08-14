# profuzz_common

`profuzz_common` is a collection of ready to use implementations for the different `traits` to be implemented to run `profuzz_core`.

Currently there are the following common implementations:

- `Healthcheck`
    - `TcpHealthcheck`: This can be used when the target has listening TCP port. The health check uses the [pcat](https://crates.io/crates/pcap) crate to listen for the response, which requires having `libpcap-dev` installed on your system. 
    - `DummyHealthcheck`: Always returns true.
- `Mutable`
    - `EtherMutatorOwned`: Implements the mutation for various network packets.
- `ResetHandler`
    - `DummyResetHandler`: Does nothing.
- `Transport`
    - `TcpTransport`: Connects to a TCP server and sends the fuzzing input over TCP.
    - `RawSocketTransport`: Sends the fuzzing input raw on the given interface.

