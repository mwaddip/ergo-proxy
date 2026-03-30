//! Ergo P2P handshake: build and parse.
//!
//! # Contract
//! - `build`: given a `HandshakeConfig`, produces raw handshake bytes.
//!   Postcondition: bytes are parseable by `parse`, and contain the configured
//!   Mode and Session features with correct encoding.
//! - `parse`: given raw bytes, extracts a `PeerSpec`.
//!   Precondition: bytes are a valid Ergo handshake payload.
//!   Postcondition: all fields populated; features parsed but unknown IDs are preserved.
//! - `validate_peer`: checks version >= 4.0.100 and session magic matches network.

use crate::transport::vlq;
use crate::types::{Network, ProxyMode, Version};
use std::io::{self, Cursor, Read};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{SystemTime, UNIX_EPOCH};

const FEATURE_MODE: u8 = 16;
const FEATURE_SESSION: u8 = 3;

/// Configuration for building a handshake.
pub struct HandshakeConfig {
    pub agent_name: String,
    pub peer_name: String,
    pub version: Version,
    pub network: Network,
    pub mode: ProxyMode,
    pub declared_address: Option<SocketAddr>,
}

/// A parsed peer feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    pub id: u8,
    pub body: Vec<u8>,
}

/// Parsed peer specification from a handshake.
#[derive(Debug, Clone)]
pub struct PeerSpec {
    pub agent: String,
    pub version: Version,
    pub name: String,
    pub address: Option<SocketAddr>,
    pub features: Vec<Feature>,
}

/// Build handshake bytes from config.
pub fn build(config: &HandshakeConfig) -> Vec<u8> {
    let mut buf = Vec::with_capacity(128);

    // Timestamp
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
    vlq::write_vlq(&mut buf, now);

    // Agent name
    vlq::write_short_string(&mut buf, &config.agent_name);

    // Version
    buf.push(config.version.major);
    buf.push(config.version.minor);
    buf.push(config.version.patch);

    // Peer name
    vlq::write_short_string(&mut buf, &config.peer_name);

    // Declared address
    match &config.declared_address {
        None => buf.push(0x00),
        Some(addr) => {
            buf.push(0x01);
            let ip_bytes = match addr.ip() {
                IpAddr::V4(ip) => ip.octets().to_vec(),
                IpAddr::V6(ip) => ip.octets().to_vec(),
            };
            buf.push((ip_bytes.len() + 4) as u8);
            buf.extend_from_slice(&ip_bytes);
            buf.extend_from_slice(&(addr.port() as u32).to_be_bytes());
        }
    }

    // Features: Mode + Session
    buf.push(2); // feature count

    // Mode feature (id=16)
    buf.push(FEATURE_MODE);
    let mode_body = build_mode_body(config.mode);
    buf.extend_from_slice(&(mode_body.len() as u16).to_be_bytes());
    buf.extend_from_slice(&mode_body);

    // Session feature (id=3)
    buf.push(FEATURE_SESSION);
    let session_body = build_session_body(config.network);
    buf.extend_from_slice(&(session_body.len() as u16).to_be_bytes());
    buf.extend_from_slice(&session_body);

    buf
}

fn build_mode_body(mode: ProxyMode) -> Vec<u8> {
    match mode {
        ProxyMode::Full => {
            // stateType=UTXO(0), verifying=true(1), nipopow=None(0), blocksToKeep=-1(zigzag=1)
            vec![0x00, 0x01, 0x00, 0x01]
        }
        ProxyMode::Light => {
            // stateType=UTXO(0), verifying=true(1), nipopow=Some(0x01) value=1(zigzag=2),
            // blocksToKeep=-1(zigzag=1)
            vec![0x00, 0x01, 0x01, 0x02, 0x01]
        }
    }
}

fn build_session_body(network: Network) -> Vec<u8> {
    let mut body = Vec::with_capacity(12);
    body.extend_from_slice(&network.magic());
    let session_id = rand_u64();
    body.extend_from_slice(&session_id.to_be_bytes());
    body
}

fn rand_u64() -> u64 {
    let mut buf = [0u8; 8];
    #[cfg(unix)]
    {
        use std::fs::File;
        let mut f = File::open("/dev/urandom").expect("/dev/urandom");
        f.read_exact(&mut buf).expect("read urandom");
    }
    #[cfg(not(unix))]
    {
        let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        buf = t.to_le_bytes();
    }
    u64::from_le_bytes(buf)
}

/// Parse handshake bytes into a PeerSpec.
pub fn parse(data: &[u8]) -> io::Result<PeerSpec> {
    let mut cursor = Cursor::new(data);

    // Timestamp (consume but don't use)
    let _timestamp = vlq::read_vlq(&mut cursor)?;

    // Agent name
    let agent = vlq::read_short_string(&mut cursor)?;

    // Version
    let mut ver = [0u8; 3];
    cursor.read_exact(&mut ver)?;
    let version = Version::new(ver[0], ver[1], ver[2]);

    // Peer name
    let name = vlq::read_short_string(&mut cursor)?;

    // Declared address
    let mut has_addr = [0u8; 1];
    cursor.read_exact(&mut has_addr)?;
    let address = if has_addr[0] != 0 {
        parse_address(&mut cursor)?
    } else {
        None
    };

    // Features
    let features = parse_features(&mut cursor)?;

    Ok(PeerSpec { agent, version, name, address, features })
}

/// Parse a peer entry from a Peers message body.
/// Same format as handshake PeerSpec but without the timestamp prefix.
pub fn parse_peer_entry(cursor: &mut Cursor<&[u8]>) -> io::Result<PeerSpec> {
    let agent = vlq::read_short_string(cursor)?;

    let mut ver = [0u8; 3];
    cursor.read_exact(&mut ver)?;
    let version = Version::new(ver[0], ver[1], ver[2]);

    let name = vlq::read_short_string(cursor)?;

    let mut has_addr = [0u8; 1];
    cursor.read_exact(&mut has_addr)?;
    let address = if has_addr[0] != 0 {
        parse_address(cursor)?
    } else {
        None
    };

    let features = parse_features(cursor)?;

    Ok(PeerSpec { agent, version, name, address, features })
}

fn parse_address<R: Read>(reader: &mut R) -> io::Result<Option<SocketAddr>> {
    let mut addr_len_byte = [0u8; 1];
    reader.read_exact(&mut addr_len_byte)?;
    let ip_len = (addr_len_byte[0] as usize).saturating_sub(4);
    let mut ip_bytes = vec![0u8; ip_len];
    reader.read_exact(&mut ip_bytes)?;
    let mut port_bytes = [0u8; 4];
    reader.read_exact(&mut port_bytes)?;
    let port = u32::from_be_bytes(port_bytes) as u16;

    match ip_len {
        4 => Ok(Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3])),
            port,
        ))),
        16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&ip_bytes);
            Ok(Some(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(octets)), port)))
        }
        _ => Ok(None),
    }
}

fn parse_features<R: Read>(reader: &mut R) -> io::Result<Vec<Feature>> {
    let mut features = Vec::new();
    let mut feat_count_buf = [0u8; 1];
    if reader.read_exact(&mut feat_count_buf).is_ok() {
        let feat_count = feat_count_buf[0] as usize;
        for _ in 0..feat_count {
            let mut fid = [0u8; 1];
            if reader.read_exact(&mut fid).is_err() {
                break;
            }
            let mut flen_bytes = [0u8; 2];
            if reader.read_exact(&mut flen_bytes).is_err() {
                break;
            }
            let flen = u16::from_be_bytes(flen_bytes) as usize;
            let mut fbody = vec![0u8; flen];
            if reader.read_exact(&mut fbody).is_err() {
                break;
            }
            features.push(Feature { id: fid[0], body: fbody });
        }
    }
    Ok(features)
}

/// Validate a peer's handshake.
///
/// # Contract
/// - **Precondition**: `spec` was produced by `parse`.
/// - **Postcondition**: returns Ok(()) if version >= 4.0.100 and session magic matches, Err otherwise.
pub fn validate_peer(spec: &PeerSpec, network: &Network) -> Result<(), String> {
    if spec.version < Version::EIP37_MIN {
        return Err(format!(
            "Peer version {} is below minimum {} (EIP-37)",
            spec.version,
            Version::EIP37_MIN
        ));
    }

    if let Some(session) = spec.features.iter().find(|f| f.id == FEATURE_SESSION) {
        if session.body.len() >= 4 {
            let peer_magic = &session.body[0..4];
            let expected = network.magic();
            if peer_magic != expected {
                return Err(format!(
                    "Session magic mismatch: {:?} (expected {:?})",
                    peer_magic, expected
                ));
            }
        }
    }

    Ok(())
}
