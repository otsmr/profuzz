use log::{debug, error};
use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{self, Config, DataLinkSender};
use pnet::packet::tcp::TcpFlags;
use pnet_layers::helper::tcp::TcpPacket;
use pnet_layers::magics::MAGIC_IPV4_TTL;
use pnet_layers::{Ether, EtherMut, Layer, LayerImmutable, LayerMut, LayerMutable, Layers};
use profuzz_core::error::ProFuzzError;
use profuzz_core::traits::HealthCheck;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A Healthcheck based on the TCP protocol, the only requirement is a listening TCP server socket. It uses the
/// behavior of the TCP protocol, that the server response with the correct ACK when the client sends a to high number.
/// In case the target crashes, the healthcheck detects that either by a TIMEOUT of 15 seconds of the server sends a
/// RST, because after the crash and a restart the target does not recognize the TCP session.
pub struct TcpHealthcheck {
    socket: TcpPacket,
    tx: Box<dyn DataLinkSender + 'static>,
    queue: RxQueue,
    running: Arc<Mutex<bool>>,
    current_tcp_sport: u16,
    current_ack: u32,
    current_ether: Option<EtherMut>,
}

impl TcpHealthcheck {
    /// Creates a new `HealthCheck` TCP instance by searching for a valid socket
    /// # Errors
    // #[must_use]
    pub fn new(iface: &str, socket: TcpPacket) -> Result<Self, ProFuzzError> {
        let queue = RxQueue::default();
        let running: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
        start_observer(iface, queue.clone(), running.clone());
        let Some(tx) = tx(iface) else {
            return Err(ProFuzzError::ConnectionFailed {
                err_msg: "Could not create tx from iface".into(),
            });
        };
        Ok(Self {
            socket,
            running,
            queue,
            tx,
            current_ack: 0x4100_4141,
            current_tcp_sport: 10_000,
            current_ether: None,
        })
    }

    fn do_tcp_handshake(&mut self) -> bool {
        self.socket.sport = self.socket.dport;
        self.current_tcp_sport += 1;
        self.current_ack += 0x01_0000;
        let Some(mut my_tcp) = self.socket.syn(self.current_ack - 1) else {
            return false;
        };
        if let Some(arp) = self.socket.arp()
            && let Some(arp) = arp.build()
            && let Some(tcp_builded) = my_tcp.clone().build()
        {
            // sending SYN
            let _ = self.tx.send_to(&tcp_builded, None);
            let _ = self.tx.send_to(&arp, None);
        }

        // giving the target 2 seconds to send SYN + ACK
        let instant = Instant::now();
        while instant.elapsed().as_secs() < 15 {
            if let Ok(mut queue) = self.queue.lock()
                && let Some(packet) = queue.pop_front()
            {
                let ether = Ether::new(&packet);
                if let Some(Layer::Tcp(tcp)) = ether.get_layer(Layers::Tcp)
                    && let Some(tcp) = tcp.as_pnet()
                {
                    debug!(
                        "Got valid packet {:x?} == {:x?}",
                        tcp.get_acknowledgement(),
                        self.current_ack
                    );
                    if tcp.get_acknowledgement() != self.current_ack {
                        continue;
                    }
                    if tcp.get_flags() & TcpFlags::RST == TcpFlags::RST {
                        debug!("Got RST packet. try again");
                        return true;
                    }
                    if let Some(LayerMut::Tcp(my_tcp)) = my_tcp.get_layer(&Layers::Tcp)
                        && let Some(mut my_tcp) = my_tcp.modify()
                    {
                        my_tcp.set_sequence(self.current_ack);
                        my_tcp.set_flags(pnet::packet::tcp::TcpFlags::ACK);
                        my_tcp.set_acknowledgement(tcp.get_sequence() + 1);
                    }
                    if let Some(tcp_builded) = my_tcp.clone().build() {
                        let _ = self.tx.send_to(&tcp_builded, None);
                    }
                    if let Some(LayerMut::Tcp(my_tcp)) = my_tcp.get_layer(&Layers::Tcp)
                        && let Some(mut my_tcp) = my_tcp.modify()
                    {
                        my_tcp.set_acknowledgement(tcp.get_sequence() + 2); // this will
                        // trigger the tcp socket to send a TCP SYN again
                    }
                    self.current_ether = Some(my_tcp.clone());

                    debug!("Got SYN ACK. Sending ACK");
                    return true;
                }
            }
            debug!("Observing...");
        }
        error!("Healthcheck run into timeout (15seconds).");

        false
    }

    fn do_healthcheck(&mut self, retries: usize) -> bool {
        if retries == 0 {
            self.current_ether = None;
            return false;
        }
        let Some(ether) = &self.current_ether else {
            return false;
        };
        if let Some(tcp_builded) = ether.clone().build() {
            let _ = self.tx.send_to(&tcp_builded, None);
            debug!(
                "Send modified ACK. Should received an TCP ACK from the target with the correct ACK."
            );
        }

        let started = Instant::now();

        while started.elapsed().as_secs() < 3 {
            if let Ok(mut queue) = self.queue.lock()
                && let Some(packet) = queue.pop_front()
            {
                let ether = Ether::new(&packet);
                if let Some(Layer::Tcp(tcp)) = ether.get_layer(Layers::Tcp)
                    && let Some(tcp) = tcp.as_pnet()
                {
                    debug!(
                        "Got valid packet {:x?} == {:x?}",
                        tcp.get_acknowledgement(),
                        self.current_ack
                    );

                    if tcp.get_acknowledgement() == 0 && tcp.get_flags() == TcpFlags::RST {
                        self.current_ether = None;
                        debug!("Target send RST. So next time create a new TCP connection.");
                        return false;
                    }

                    if tcp.get_acknowledgement() != self.current_ack {
                        continue;
                    }
                    debug!("Got ACK, target is still alive.");
                    return true;
                }
            }
        }
        self.do_healthcheck(retries - 1)
    }
}

impl HealthCheck for TcpHealthcheck {
    async fn is_ok(&mut self) -> Result<bool, profuzz_core::error::ProFuzzError> {
        if self.current_ether.is_some() {
            Ok(self.do_healthcheck(5))
        } else {
            debug!("Doing handshake.");
            Ok(self.do_tcp_handshake())
        }
    }
}

impl Drop for TcpHealthcheck {
    fn drop(&mut self) {
        if let Ok(mut run) = self.running.lock() {
            *run = false;
        }
        debug!("Stopping TCP HealthCheck: {:#?}", self.socket);
    }
}

pub(crate) type RxQueue = Arc<Mutex<VecDeque<Vec<u8>>>>;

pub(crate) fn tx(iface: &str) -> Option<Box<dyn DataLinkSender>> {
    let interfaces = datalink::interfaces();
    match interfaces.into_iter().find(|x| x.name == iface) {
        Some(interface) => match datalink::channel(&interface, Config::default()) {
            Ok(Ethernet(tx, _)) => Some(tx),
            _ => None,
        },
        None => None,
    }
}

fn start_observer(iface: &str, queue: RxQueue, running: Arc<Mutex<bool>>) {
    let mut device = None;
    for cur_device in pcap::Device::list().expect("device lookup failed") {
        if cur_device.name == iface {
            device = Some(cur_device);
            break;
        }
    }
    let Some(device) = device else {
        error!("Observer iface {iface} not found");
        return;
    };

    let Ok(tmp) = pcap::Capture::from_device(device) else {
        error!("Could not get Capture device");
        return;
    };

    let tmp = tmp.timeout(100);

    let mut cap = match tmp.immediate_mode(true).open() {
        Ok(a) => a,
        Err(e) => {
            error!("Capture err: {e}.");
            return;
        }
    };
    std::thread::spawn(move || {
        loop {
            match running.lock() {
                Ok(running) => {
                    if !*running {
                        return;
                    }
                }
                Err(_) => return,
            }
            if let Ok(packet) = cap.next_packet() {
                let eth = Ether::new(packet.data);

                if let Some(Layer::Ipv4(ipv4)) = eth.get_layer(Layers::Ipv4)
                    && let Some(ipv4) = ipv4.as_pnet()
                    && ipv4.get_ttl() != MAGIC_IPV4_TTL
                    && let Some(Layer::Tcp(_)) = eth.get_layer(Layers::Tcp)
                    && let Ok(mut q) = queue.lock()
                {
                    q.push_back(packet.to_vec());
                }
            }
        }
    });
}

#[tokio::test]
async fn test_tcp_healthcheck() {
    use pnet::util::MacAddr;
    use std::net::Ipv4Addr;
    use std::str::FromStr;
    use std::thread::sleep;
    use std::time::Duration;

    pretty_env_logger::init();
    let socket = TcpPacket {
        eth_src: MacAddr::from_str("33:33:33:33:33:33").expect(""),
        eth_dst: MacAddr::from_str("33:33:33:33:33:33").expect(""),
        vlan_id: Some(10),
        ipv4_dst: Ipv4Addr::from_str("10.10.22.33").expect(""),
        ipv4_src: Ipv4Addr::from_str("10.10.22.33").expect(""),
        dport: 41000,
        sport: 1000,
    };

    let mut healthcheck = TcpHealthcheck::new("eth0", socket).expect("");

    for _ in 0..30 {
        assert!(healthcheck.is_ok().await.expect(""));
        sleep(Duration::from_millis(1000));
    }
}
