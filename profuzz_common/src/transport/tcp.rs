use profuzz_core::error::ProFuzzError;
use std::fmt::Display;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::time::timeout;

use profuzz_core::traits::Transport;

/// Tcp Transporter config
#[derive(Default, Clone)]
pub struct TcpConfig {
    /// read timeout in millis
    /// if the read timeout is None, then it will not be read
    pub read_timeout: Option<u64>,
    /// write timeout in millis
    pub write_timeout: u64,
}

/// Basic TCP Client Transport Layer
pub struct TcpTransport<T> {
    addr: T,
    stream: Option<TcpStream>,
    config: TcpConfig,
    send_after_connected: Option<Vec<Vec<u8>>>,
}

impl<T: ToSocketAddrs> TcpTransport<T> {
    /// Create a instance of Tcp Transport
    pub fn new(addr: T, config: TcpConfig, send_after_connected: Option<Vec<Vec<u8>>>) -> Self {
        Self {
            addr,
            stream: None,
            config,
            send_after_connected,
        }
    }
}

impl<T: ToSocketAddrs + Display> Transport for TcpTransport<T> {
    fn title(&self) -> String {
        format!("tcp_client ({})", self.addr)
    }

    async fn connect(&mut self) -> Result<(), ProFuzzError> {
        let stream = TcpStream::connect(&self.addr).await?;
        let _ = stream.set_nodelay(true);
        self.stream = Some(stream);

        if let Some(send_after_connected) = self.send_after_connected.clone() {
            let mut tmp = [0u8; 3000];
            for message in &send_after_connected {
                self.write(message).await?;
                self.read(&mut tmp).await?;
            }
        }

        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProFuzzError> {
        let stream = self.stream.take();
        if let Some(mut stream) = stream {
            let _ = stream.flush().await;
            let _ = stream.shutdown().await;
        }
        Ok(())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, ProFuzzError> {
        if let Some(stream) = self.stream.as_mut() {
            if let Some(read_timeout) = self.config.read_timeout {
                Ok(timeout(Duration::from_millis(read_timeout), stream.read(buf)).await??)
            } else {
                Ok(0)
            }
        } else {
            Err(ProFuzzError::ConnectionFailed {
                err_msg: "tcp stream not connected".into(),
            })
        }
    }

    async fn write(&mut self, buf: &[u8]) -> Result<(), ProFuzzError> {
        if let Some(stream) = self.stream.as_mut() {
            timeout(
                Duration::from_millis(self.config.write_timeout),
                stream.write_all(buf),
            )
            .await??;
            Ok(())
        } else {
            Err(ProFuzzError::ConnectionFailed {
                err_msg: "tcp stream not connected".into(),
            })
        }
    }
}
