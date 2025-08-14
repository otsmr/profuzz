# profuzz

`profuzz` is a generic approach to easily create a fast and easy-to use protocol fuzzer for custom targets. `profuzz` aims to be used mainly in the embedded world, where most of the time it is not easy to create a running harness on a Linux-based system because of hardware dependencies, the source code is not available, or it requires hardware attacks to dump the firmware. Dumping the firmware, reverse engineering, and then fuzzing potential targets is time intensive. In these cases `profuzz` can be used to find "low-hanging" fruits by fuzzing either the network stack itself or custom binary protocols.

![screenshot](images/screenshot.png)

## The generic architecture
![screenshot](images/structure.png)

The `profuzz_core` highlighted in **blue** contains the main fuzzing loop, a TUI, and a CLI. It is basically the "glue" code for the target-specific crate and the implementations of all `traits` highlighted in **yellow** required to use the `profuzz_core` crate. These traits implementing the logic to communicate with the target, mutate the corpus, resetting the target or to perform a health check. In the crate `profuzz_common` multiple implementations can be found and reused for different target-specific fuzzing setups.

## Getting started

The `main` function to start `profuzz_core` must be implemented by the target-specific crate. This allows `profuzz_core` to be used either in a small setup, as shown in the [example/profuzz_network_stack](example/profuzz_network_stack/) or in a bigger project, like an automated scanning tool as one feature in many to test multiple targets.

The following code shows the basic setup required to create a network stack fuzzer. It uses a TCP server on the target to detect a crash and sends the packets directly on eth0.

The full example can be found in the [example](example) folder.

```rs
#[tokio::main]
async fn main() {

    // Defining the `Transport` crate by using a raw linux socket provided by the profuzz_common crate.
    let transport = RawSocketTransport::new("eth0");

    // Defining the `Healthcheck` using a TCP server. The implementation is also provided by the profuzz_common crate.
    let healthcheck = TcpHealthcheck::new(
       "lo0",
       TcpPacket {
           eth_src: MacAddr::from_str("13:33:33:33:33:37").unwrap(),
           eth_dst: MacAddr::from_str("13:33:33:33:33:38").unwrap(),
           vlan_id: None,
           ipv4_src: Ipv4Addr::from([127, 0, 0, 7]),
           ipv4_dst: Ipv4Addr::from([127, 0, 0, 8]),
           sport: 1337,
           dport: 1338,
       },
   )
   .unwrap();

    // Initialization the `profuzz_core` crate by providing the different implementations for the traits
    let fuzzer = ProFuzzer::new(transport, healthcheck, DummyResetHandler());

    // Starting the CLI including a TUI, and defining the `Mutable` implementation struct that
    // implements the mutation of the corpus files also provided by the `profuzz_common` crate
    if let Err(err) = fuzzer.start_cli::<EtherMutatorOwned>().await {
        eprintln!("{err}");
    }
}
```

## Using the CLI to start the fuzzer

In case the `start_cli` function is used to start the fuzzer the following options are available at the moment:

```plain
Usage: profuzz_network_stack [OPTIONS] <COMMAND>

Commands:
  triage  Triage found crashes to identify the potential root cause
  fuzz    
  help    Print this message or the help of the given subcommand(s)

Options:
      --verbose  Verbose mode
  -h, --help     Print help
```

### Start fuzzing

To start the fuzzer the `fuzz` command can be used with the following options. When started `profuzz_core` automatically create an output directory storing all detected `crashes`.

```plain
Usage: profuzz_network_stack fuzz [OPTIONS] --in-dir <IN_DIR> --out-dir <OUT_DIR>

Options:
  -i, --in-dir <IN_DIR>    input directory with test cases
  -o, --out-dir <OUT_DIR>  output directory for fuzzer findings
      --hide-ui            Displays the profuzz UI
      --auto-resume        If output directory is not empty auto resume the session
  -h, --help               Print help
```

### Triaging a crash

When a crash is detected, e.g., the health check reports the target is not healthy `profuzz_core` stores all messages sent to the target since the last successful health check. The `triage` command then resends all the buffers while performing a health check after each send buffer. In case the health check reports unhealthy, the crash is detected and stored in the `<out-dir>/crashes/<sha1>`.

```plain
Usage: profuzz_network_stack triage --out-dir <OUT_DIR>

Options:
  -o, --out-dir <OUT_DIR>  output directory for fuzzer findings
  -h, --help               Print help
```

# License
This project is licensed under the [Apache-2.0](./LICENSE) license.