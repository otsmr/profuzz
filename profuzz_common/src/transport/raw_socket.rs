use pnet::datalink::Channel::Ethernet;
use pnet::datalink::Config;
use pnet::datalink::{self, DataLinkSender};
use profuzz_core::error::ProFuzzError;

use profuzz_core::traits::Transport;

/// Default transport layer for sending directly to an socket.
/// ! This transport layer requires root permissions when executing!
/// ! This transport DOES not read from the socket, so now feedback based fuzzing is possible
pub struct RawSocketTransport {
    iface: String,
    socket: Option<Box<dyn DataLinkSender>>,
}

impl RawSocketTransport {
    /// Create a instance of `RawSocketTransport`
    #[must_use]
    pub fn new(iface: &str) -> Self {
        Self {
            iface: iface.to_owned(),
            socket: None,
        }
    }
}

impl Transport for RawSocketTransport {
    fn title(&self) -> String {
        format!("raw_socket ({})", self.iface)
    }

    async fn connect(&mut self) -> Result<(), ProFuzzError> {
        if self.socket.is_some() {
            return Ok(());
        }
        let interfaces = datalink::interfaces();
        let tx = match interfaces.into_iter().find(|x| x.name == self.iface) {
            Some(interface) => match datalink::channel(&interface, Config::default()) {
                Ok(Ethernet(tx, _)) => Ok(tx),
                a => Err(ProFuzzError::ConnectionFailed {
                    err_msg: format!("{:?}", a.err()),
                }),
            },
            None => Err(ProFuzzError::ConnectionFailed {
                err_msg: format!("Interface {} not found!", self.iface),
            }),
        }?;
        self.socket = Some(tx);
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProFuzzError> {
        // let _ = self.socket.take();
        Ok(())
    }

    async fn read(&mut self, _buf: &mut [u8]) -> Result<usize, ProFuzzError> {
        Ok(0)
    }

    async fn write(&mut self, buf: &[u8]) -> Result<(), ProFuzzError> {
        if let Some(socket) = self.socket.as_mut() {
            if let Some(err) = socket.send_to(buf, None) {
                err?;
            }
            Ok(())
        } else {
            Err(ProFuzzError::ConnectionFailed {
                err_msg: "raw socket not found".into(),
            })
        }
    }
}
