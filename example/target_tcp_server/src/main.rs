#![allow(unexpected_cfgs)]

use pnet_macros::packet;
use pnet_macros_support::packet::Packet;
use pnet_macros_support::types::{u16be, u32be};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[packet]
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

#[derive(Default)]
struct Context {
    last_time_crash_msg_received: Option<Instant>,
    counter_last_time_crash_msg_received: usize,
}

fn handle_client(mut stream: TcpStream, context: Arc<Mutex<Context>>) {
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size == 0 {
                    break;
                }
                if size == 4 && buffer[..size] == [1, 0, 1, 0] {
                    let response = [1u8, 0, 2, 0];
                    stream
                        .write_all(&response)
                        .expect("Failed to write to stream");
                    // close TCP connection...
                    return;
                }
                if let Some(tether) = TetherPacket::new(&buffer[..size]) {
                    if size < 20 {
                        let response = b"Nop";
                        stream
                            .write_all(response)
                            .expect("Failed to write to stream");
                        // close TCP connection...
                        return;
                    }

                    let checksum = tether_checksum(tether.packet().to_vec());
                    if checksum != tether.get_crc32() {
                        let response = b"Nop";
                        stream
                            .write_all(response)
                            .expect("Failed to write to stream");
                        return;
                        // println!("Checksum is not valid");
                        // packet is just dropped
                        // continue;
                    }

                    // println!("function_id: {}", tether.get_function_id());
                    let response = match tether.get_function_id() {
                        0 => [0u8; 10],
                        1 => [1u8; 10],
                        2 => {
                            // println!("{}", tether.get_options());
                            if tether.get_options() == 65275 {
                                // println!("{:?}", tether.payload());
                                if !tether.payload().is_empty() && tether.payload()[0] == 242 {
                                    println!("\n\n > CRASH 3 < \n\n");
                                    std::process::exit(1);
                                }
                                [5u8; 10]
                            } else {
                                [2u8; 10]
                            }
                        }
                        3 => {
                            // println!("{}", tether.get_options());
                            if tether.get_options() == 279 {
                                println!("\n\n > CRASH 1 < \n\n");
                                std::process::exit(1);
                            }
                            [0x3u8; 10]
                        }
                        4 => {
                            if let Ok(context) = context.lock().as_mut() {
                                if let Some(last) = context.last_time_crash_msg_received {
                                    if last.elapsed().as_secs() < 1 {
                                        context.last_time_crash_msg_received = Some(Instant::now());
                                        context.counter_last_time_crash_msg_received += 1;
                                    } else {
                                        context.counter_last_time_crash_msg_received = 0;
                                    }
                                }
                                println!("{}", context.counter_last_time_crash_msg_received);
                                context.last_time_crash_msg_received = Some(Instant::now());
                                if context.counter_last_time_crash_msg_received >= 11 {
                                    println!("\n\n > CRASH 2 < \n\n");
                                    std::process::exit(1);
                                }
                            }
                            [0x8u8; 10]
                        }
                        _ => [9u8; 10],
                    };
                    stream
                        .write_all(&response)
                        .expect("Failed to write to stream");
                } else {
                    println!("Got invalid packet!");
                    continue;
                }
            }
            Err(e) => {
                eprintln!("Failed to read from client: {e}");
                return;
            }
        }
    }
}

fn main() {
    let addr = "127.0.0.1:20002";
    let listener = TcpListener::bind(addr).expect("Could not bind to address");

    println!("Server listening on {addr}");

    let context = Context::default();
    let context = Arc::new(Mutex::new(context));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let context = context.clone();
                // thread::spawn(move || {
                handle_client(stream, context);
                // });
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {e}");
            }
        }
    }
}
