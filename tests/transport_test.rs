mod common;

use ergo_proxy_node::transport::vlq;

#[test]
fn vlq_encode_zero() {
    let mut buf = Vec::new();
    vlq::write_vlq(&mut buf, 0);
    assert_eq!(buf, vec![0x00]);
}

#[test]
fn vlq_encode_single_byte() {
    let mut buf = Vec::new();
    vlq::write_vlq(&mut buf, 127);
    assert_eq!(buf, vec![0x7f]);
}

#[test]
fn vlq_encode_two_bytes() {
    let mut buf = Vec::new();
    vlq::write_vlq(&mut buf, 128);
    assert_eq!(buf, vec![0x80, 0x01]);
}

#[test]
fn vlq_encode_large_value() {
    // 300 = 0b100101100 → low 7: 0101100 = 0x2c | 0x80, high 7: 0000010 = 0x02
    let mut buf = Vec::new();
    vlq::write_vlq(&mut buf, 300);
    assert_eq!(buf, vec![0xac, 0x02]);
}

#[test]
fn vlq_roundtrip() {
    let values = [0u64, 1, 127, 128, 255, 256, 16383, 16384, 1_000_000, u64::MAX >> 1];
    for &val in &values {
        let mut buf = Vec::new();
        vlq::write_vlq(&mut buf, val);
        let mut cursor = std::io::Cursor::new(buf.as_slice());
        let decoded = vlq::read_vlq(&mut cursor).unwrap();
        assert_eq!(decoded, val, "roundtrip failed for {}", val);
    }
}

#[test]
fn vlq_read_overflow_returns_error() {
    let data = vec![0x80; 10];
    let mut cursor = std::io::Cursor::new(data.as_slice());
    assert!(vlq::read_vlq(&mut cursor).is_err());
}

#[test]
fn vlq_length_rejects_oversized() {
    let mut buf = Vec::new();
    vlq::write_vlq(&mut buf, 300_000);
    let mut cursor = std::io::Cursor::new(buf.as_slice());
    assert!(vlq::read_vlq_length(&mut cursor).is_err());
}

#[test]
fn vlq_length_accepts_valid() {
    let mut buf = Vec::new();
    vlq::write_vlq(&mut buf, 1024);
    let mut cursor = std::io::Cursor::new(buf.as_slice());
    assert_eq!(vlq::read_vlq_length(&mut cursor).unwrap(), 1024);
}

#[test]
fn zigzag_encode_positive() {
    assert_eq!(vlq::zigzag_encode(1), 2);
    assert_eq!(vlq::zigzag_encode(2), 4);
    assert_eq!(vlq::zigzag_encode(1440), 2880);
}

#[test]
fn zigzag_encode_negative() {
    assert_eq!(vlq::zigzag_encode(-1), 1);
    assert_eq!(vlq::zigzag_encode(-2), 3);
}

#[test]
fn zigzag_roundtrip() {
    for val in [-1000i32, -2, -1, 0, 1, 2, 1000, i32::MAX, i32::MIN] {
        let encoded = vlq::zigzag_encode(val);
        let decoded = vlq::zigzag_decode(encoded);
        assert_eq!(decoded, val, "zigzag roundtrip failed for {}", val);
    }
}
