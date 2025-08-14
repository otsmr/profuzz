#![allow(unexpected_cfgs)]

use std::collections::HashSet;

use pnet::packet::MutablePacket;
use pnet_macros::packet;
use pnet_macros_support::types::{u16be, u32be};
use pnet_show::Show;
use profuzz_common::transport::tcp::{TcpConfig, TcpTransport};
use profuzz_core::cli::ProFuzzBuilder;
use profuzz_core::error::ProFuzzError;
use profuzz_core::mutator::Mutator;
use profuzz_core::traits::{Corpus, HealthCheck, Mutable};
use profuzz_core::traits::{ResetHandler, Transport};

#[packet]
#[derive(Show)]
pub struct Tether {
    version: u8,
    unknown0: u8,
    tether_type: u8,
    unknown1: u8,
    length: u16be,
    unknown2: u16be,
    unknown3: u32be,
    crc32: u32be,
    options: u16be,
    function_id: u16be,
    #[payload]
    payload: Vec<u8>,
}

fn tether_checksum(mut bytes: Vec<u8>) -> u32be {
    if bytes.len() < 16 {
        return 0;
    }
    bytes[12] = 0x5A;
    bytes[13] = 0x6B;
    bytes[14] = 0x7C;
    bytes[15] = 0x8D;
    let crc32 = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
    let crc = crc32.checksum(&bytes).to_le();
    crc as u32be
}

#[derive(Clone, Debug)]
struct TetherLayer {
    buf: Vec<u8>,
}

impl Corpus for TetherLayer {
    fn from_bytes(buf: Vec<u8>) -> Option<Self> {
        if let Some(pnet) = TetherPacket::new(&buf)
            && pnet.get_tether_type() == 5
        {
            return Some(TetherLayer { buf });
        }
        None
    }

    fn to_bytes(self) -> Vec<u8> {
        self.buf.to_vec()
    }
    fn show(&self) -> String {
        if let Some(pnet) = TetherPacket::new(&self.buf) {
            pnet.show()
            // if pnet.get_tether_type() == 5 {
            //     return Some(TetherLayer { buf });
            // }
        } else {
            String::new()
        }
    }

    fn build(mut self) -> Vec<u8> {
        let mut len = self.buf.len();
        if len > 16 {
            len -= 16;
        } else {
            len = 0;
        }
        if let Some(mut pnet) = MutableTetherPacket::new(&mut self.buf)
            && pnet.get_tether_type() == 5
        {
            pnet.set_length(len as u16);
        }
        let checksum = tether_checksum(self.buf.clone());
        if let Some(mut pnet) = MutableTetherPacket::new(&mut self.buf)
            && pnet.get_tether_type() == 5
        {
            pnet.set_crc32(checksum);
        }
        self.buf
    }
}

impl Mutable for TetherLayer {
    fn mutate<R: rand::Rng>(&mut self, mutator: &mut Mutator<R>) {
        let mut len = self.buf.len();
        if len > 16 {
            len -= 16;
        } else {
            len = 0;
        }
        // if mutator.gen_chance(0.001) {
        //     self.buf.resize(mutator.gen_range(self.buf.len(), 1024), 0);
        // }
        if let Some(mut pnet) = MutableTetherPacket::new(&mut self.buf)
            && pnet.get_tether_type() == 5
        {
            if mutator.gen_chance(0.999) {
                pnet.set_length(len as u16);
            }
            if mutator.gen_chance(0.01) {
                let mut mutable = pnet.get_function_id();
                mutator.mutate(&mut mutable);
                // 2560 = resetting the TP Link Router. So ignoring it
                let ignore_function_ids = HashSet::from([200]);
                while ignore_function_ids.contains(&mutable) || mutable > 100 {
                    mutator.mutate(&mut mutable);
                }
                pnet.set_function_id(mutable);
            }
            if mutator.gen_chance(0.01) {
                let mut mutable = pnet.get_options();
                mutator.mutate(&mut mutable);
                pnet.set_options(mutable);
            }
            let mut payload_chance = (100 / (len - 2)) as f64;
            if payload_chance <= 0.1 {
                payload_chance = 0.1;
            }
            for byte in pnet.payload_mut() {
                if mutator.gen_chance(payload_chance) {
                    mutator.mutate(byte);
                }
            }
        }
        let checksum = tether_checksum(self.buf.clone());
        if let Some(mut pnet) = MutableTetherPacket::new(&mut self.buf)
            && pnet.get_tether_type() == 5
            && mutator.gen_chance(0.9999)
        {
            pnet.set_crc32(checksum);
        }
    }
}

struct TetherHealthCheck {
    transport: TcpTransport<&'static str>,
}

impl HealthCheck for TetherHealthCheck {
    async fn is_ok(&mut self) -> Result<bool, ProFuzzError> {
        self.transport.connect().await?;
        self.transport.write(&[0x01, 0x00, 0x01, 0x00]).await?;
        let mut buffer = [0u8; 100];
        let size = self.transport.read(&mut buffer).await?;
        if size == 4 && buffer[..size] == [0x01, 0x00, 0x02, 0x00] {
            return Ok(true);
        }
        Ok(false)
    }
}

struct TetherResetHandler;

impl ResetHandler for TetherResetHandler {
    async fn reset(&mut self) -> Result<(), ProFuzzError> {
        std::fs::write("./example/target_tcp_server/reseted.txt", "1")
            .expect("Could not write the reset file");
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let tcp_config = TcpConfig {
        read_timeout: Some(500), // target does not always return something...
        write_timeout: 500,
    };

    let addr = "127.0.0.1:20002";

    let transport = TcpTransport::new(addr, tcp_config.clone(), None);
    let healthcheck = TetherHealthCheck { transport };

    let send_after_connected = vec![
        // vec![1, 0, 1, 0],
        // hex::decode("01000200010005000008000000000001f807042701010f0000000000").expect(""),
    ];

    let transport = TcpTransport::new(addr, tcp_config, Some(send_after_connected));

    let resethandler = TetherResetHandler;

    let fuzzer = ProFuzzBuilder::new(transport, healthcheck, resethandler);
    if let Err(err) = fuzzer.start_cli::<TetherLayer>().await {
        eprintln!("{err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::{BufReader, Read};

    fn load_initial_corpus() -> Vec<TetherLayer> {
        let corpuses = match fs::read_dir("./corpus") {
            Ok(f) => f,
            Err(_) => fs::read_dir("./profuzz_tplink_tmdp/corpus/").unwrap(),
        };

        let mut initial_corpus = vec![];

        for corpus in corpuses {
            let corpus = corpus.unwrap();
            if !corpus.file_type().unwrap().is_file() {
                continue;
            }
            let file = File::open(corpus.path()).unwrap();
            let mut buffer = Vec::new();
            BufReader::new(file).read_to_end(&mut buffer).unwrap();
            if let Some(pnet) = TetherPacket::new(&buffer) {
                let is = pnet.get_crc32() == tether_checksum(buffer.clone());
                if pnet.get_tether_type() == 5 && is {
                    initial_corpus.push(TetherLayer::from_bytes(buffer).unwrap());
                } else {
                    println!("Error parsing: {corpus:?}");
                }
            } else {
                println!("Error parsing: {corpus:?}");
                // initial_corpus.push(TetherLayer::from_bytes(buffer));
            }
        }
        initial_corpus
    }
    #[test]
    fn test_checksum() {
        let corpuses = load_initial_corpus();
        for mut corpus in corpuses {
            let pnet = MutableTetherPacket::new(&mut corpus.buf).unwrap();
            let checksum = pnet.get_crc32();
            assert_eq!(checksum, tether_checksum(corpus.buf))
        }
    }
}
