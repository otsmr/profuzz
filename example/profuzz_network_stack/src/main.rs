use pnet::util::MacAddr;
use pnet_layers::helper::tcp::TcpPacket;
use pnet_layers::{EtherMut, Ipv4Mut, LayerMut, LayerMutable, PayloadMut, UdpMut, VlanMut};
use profuzz_common::healthcheck::tcp::TcpHealthcheck;
use profuzz_common::mutable::pnet::constraints::{
    EthPayload, Rpv, RulePacket, RuleVlan, VlanPayload,
};
use profuzz_common::mutable::pnet::{EtherMutatorOwned, set_mutation_constraints};
use profuzz_common::resethandler::dummy::DummyResetHandler;
use profuzz_common::transport::raw_socket::RawSocketTransport;
use profuzz_core::cli::ProFuzzBuilder;
use std::fs::File;
use std::io::Write;
use std::net::Ipv4Addr;
use std::str::FromStr;

/// Create a sample corpus packet
fn get_example_corpus() -> EtherMut {
    let mut random = EtherMut::new();
    {
        let mut eth = random.modify().unwrap();
        eth.set_source(MacAddr::from_str("3c:ce:33:33:33:33").unwrap());
        eth.set_destination(MacAddr::broadcast());
    }
    random.add(LayerMut::Vlan(VlanMut::new()));
    random.add(LayerMut::Ipv4(Ipv4Mut::new()));
    random.add(LayerMut::Udp(UdpMut::new()));
    random.add(LayerMut::Payload(
        PayloadMut::from_buf(vec![0; 10]).unwrap(),
    ));
    random
}

#[tokio::main]
async fn main() {
    std::fs::create_dir_all("profuzz_pnet/corpus/").unwrap();
    let mut file = File::create("profuzz_pnet/corpus/example.bin").unwrap();
    file.write_all(&get_example_corpus().build().unwrap())
        .unwrap();

    // Define the transport. In this case it uses the raw socket to send the corpus files as the
    // whole network stack should be fuzzed.
    let transport = RawSocketTransport::new("en0");

    // Define a healthcheck. In this case the common TCP healthcheck is used and the TCP server
    // used to check the target is defined.
    let healthcheck = TcpHealthcheck::new(
        "en1", // If possible it is recommended to use a different interface than used for the Transport
        TcpPacket {
            eth_src: MacAddr::from_str("32:a4:e7:9a:c7:99").unwrap(),
            eth_dst: MacAddr::from_str("32:a4:e7:9a:c7:8a").unwrap(),
            vlan_id: None,
            ipv4_src: Ipv4Addr::from([127, 0, 0, 2]),
            ipv4_dst: Ipv4Addr::from([127, 0, 0, 1]),
            dport: 1337,
            sport: 1330,
        },
    )
    .unwrap();

    // As mutation the common implementation `EtherMutatorOwned` is used. This allows to set
    // constraints to the fuzzing. In this case the only constraint is that the VLAN IDs must be
    // either 13 or 37 every other field can be modifid.
    set_mutation_constraints(vec![RulePacket {
        eth_saddr: Rpv::Any,
        eth_daddr: Rpv::Any,
        eth_type: Rpv::Any,
        eth_payload: EthPayload::Vlan(RuleVlan {
            id: Rpv::Contains(vec![13, 37]),
            payload: VlanPayload::Any,
        }),
    }])
    .expect("Could not set constraints.");

    // Initialize the engine with all the different implementation. For the reset handler a dummy
    // is used.
    let fuzzer = ProFuzzBuilder::new(transport, healthcheck, DummyResetHandler());

    // Starting in the cli mode, so the user can start either the fuzzer or the triaging.
    if let Err(err) = fuzzer.start_cli::<EtherMutatorOwned>().await {
        eprintln!("{err}");
    }
}
