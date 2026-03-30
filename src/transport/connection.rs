//! Per-connection async read/write loops.
//!
//! # Contract
//! - `Connection::outbound`: performs handshake as initiator (send then receive).
//!   Precondition: `stream` is a connected TCP stream.
//!   Postcondition: both sides have exchanged handshakes; peer's PeerSpec is available.
//! - `Connection::inbound`: performs handshake as responder (receive then send).
//!   Precondition: `stream` is a connected TCP stream.
//!   Postcondition: both sides have exchanged handshakes; peer's PeerSpec is available.
//! - `Connection::read_frame`: reads the next frame from the stream.
//!   Postcondition: returns a validated `Frame` or error.
//! - `Connection::write_frame`: writes a frame to the stream.
//!   Postcondition: frame is fully written and flushed.
//! - Invariant: the connection's magic is fixed at construction time and never changes.

use crate::transport::frame::{self, Frame};
use crate::transport::handshake::{self, HandshakeConfig, PeerSpec};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
const HANDSHAKE_BUF_SIZE: usize = 8192;

/// An active P2P connection with completed handshake.
pub struct Connection {
    stream: TcpStream,
    magic: [u8; 4],
    peer_spec: PeerSpec,
}

impl Connection {
    /// Establish a connection by performing the handshake as initiator (outbound).
    ///
    /// Sends our handshake, reads the peer's handshake, validates it.
    pub async fn outbound(
        mut stream: TcpStream,
        config: &HandshakeConfig,
    ) -> io::Result<Self> {
        let magic = config.network.magic();

        // Send our handshake
        let hs_bytes = handshake::build(config);
        stream.write_all(&hs_bytes).await?;
        stream.flush().await?;

        // Read peer's handshake
        let peer_spec = timeout(HANDSHAKE_TIMEOUT, read_handshake(&mut stream))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Handshake timeout"))??;

        // Validate
        handshake::validate_peer(&peer_spec, &config.network)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(Self { stream, magic, peer_spec })
    }

    /// Accept a connection by performing the handshake as responder (inbound).
    ///
    /// Reads the peer's handshake first, validates it, then sends ours.
    pub async fn inbound(
        mut stream: TcpStream,
        config: &HandshakeConfig,
    ) -> io::Result<Self> {
        let magic = config.network.magic();

        // Read peer's handshake first
        let peer_spec = timeout(HANDSHAKE_TIMEOUT, read_handshake(&mut stream))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Handshake timeout"))??;

        // Validate before sending ours
        handshake::validate_peer(&peer_spec, &config.network)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Send our handshake
        let hs_bytes = handshake::build(config);
        stream.write_all(&hs_bytes).await?;
        stream.flush().await?;

        Ok(Self { stream, magic, peer_spec })
    }

    /// Read the next message frame.
    pub async fn read_frame(&mut self) -> io::Result<Frame> {
        frame::read_frame(&mut self.stream, &self.magic).await
    }

    /// Write a message frame.
    pub async fn write_frame(&mut self, f: &Frame) -> io::Result<()> {
        frame::write_frame(&mut self.stream, &self.magic, f).await
    }

    /// Get the peer's specification from the handshake.
    pub fn peer_spec(&self) -> &PeerSpec {
        &self.peer_spec
    }

    /// Split the connection into separate read and write halves.
    pub fn split(self) -> (tokio::net::tcp::OwnedReadHalf, tokio::net::tcp::OwnedWriteHalf, [u8; 4], PeerSpec) {
        let (read, write) = self.stream.into_split();
        (read, write, self.magic, self.peer_spec)
    }
}

async fn read_handshake(stream: &mut TcpStream) -> io::Result<PeerSpec> {
    let mut buf = vec![0u8; HANDSHAKE_BUF_SIZE];
    tokio::time::sleep(Duration::from_millis(200)).await;
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::ConnectionReset, "Empty handshake"));
    }
    handshake::parse(&buf[..n])
}
