use ergo_proxy_node::protocol::messages::{ProtocolMessage, MessageCode};
use ergo_proxy_node::transport::frame::Frame;
use ergo_proxy_node::transport::vlq;

#[test]
fn parse_inv_message() {
    let mut body = Vec::new();
    body.push(2);
    vlq::write_vlq(&mut body, 1);
    body.extend_from_slice(&[0xaa; 32]);

    let frame = Frame { code: MessageCode::INV, body };
    let msg = ProtocolMessage::from_frame(&frame).unwrap();

    match msg {
        ProtocolMessage::Inv { modifier_type, ids } => {
            assert_eq!(modifier_type, 2);
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0], [0xaa; 32]);
        }
        _ => panic!("Expected Inv"),
    }
}

#[test]
fn parse_modifier_request() {
    let mut body = Vec::new();
    body.push(2);
    vlq::write_vlq(&mut body, 2);
    body.extend_from_slice(&[0xbb; 32]);
    body.extend_from_slice(&[0xcc; 32]);

    let frame = Frame { code: MessageCode::MODIFIER_REQUEST, body };
    let msg = ProtocolMessage::from_frame(&frame).unwrap();

    match msg {
        ProtocolMessage::ModifierRequest { modifier_type, ids } => {
            assert_eq!(modifier_type, 2);
            assert_eq!(ids.len(), 2);
        }
        _ => panic!("Expected ModifierRequest"),
    }
}

#[test]
fn parse_modifier_response() {
    let mut body = Vec::new();
    body.push(2);
    vlq::write_vlq(&mut body, 1);
    body.extend_from_slice(&[0xdd; 32]);
    vlq::write_vlq(&mut body, 4);
    body.extend_from_slice(&[1, 2, 3, 4]);

    let frame = Frame { code: MessageCode::MODIFIER_RESPONSE, body };
    let msg = ProtocolMessage::from_frame(&frame).unwrap();

    match msg {
        ProtocolMessage::ModifierResponse { modifier_type, modifiers } => {
            assert_eq!(modifier_type, 2);
            assert_eq!(modifiers.len(), 1);
            assert_eq!(modifiers[0].0, [0xdd; 32]);
            assert_eq!(modifiers[0].1, vec![1, 2, 3, 4]);
        }
        _ => panic!("Expected ModifierResponse"),
    }
}

#[test]
fn parse_get_peers() {
    let frame = Frame { code: MessageCode::GET_PEERS, body: vec![] };
    let msg = ProtocolMessage::from_frame(&frame).unwrap();
    assert!(matches!(msg, ProtocolMessage::GetPeers));
}

#[test]
fn parse_sync_info_is_opaque() {
    let body = vec![0x01, 0x02, 0x03, 0x04, 0x05];
    let frame = Frame { code: MessageCode::SYNC_INFO, body: body.clone() };
    let msg = ProtocolMessage::from_frame(&frame).unwrap();
    match msg {
        ProtocolMessage::SyncInfo { body: b } => assert_eq!(b, body),
        _ => panic!("Expected SyncInfo"),
    }
}

#[test]
fn parse_unknown_code_preserved() {
    let frame = Frame { code: 99, body: vec![0xff] };
    let msg = ProtocolMessage::from_frame(&frame).unwrap();
    match msg {
        ProtocolMessage::Unknown { code, body } => {
            assert_eq!(code, 99);
            assert_eq!(body, vec![0xff]);
        }
        _ => panic!("Expected Unknown"),
    }
}

#[test]
fn inv_to_frame_roundtrip() {
    let mut ids_body = Vec::new();
    ids_body.push(2);
    vlq::write_vlq(&mut ids_body, 1);
    ids_body.extend_from_slice(&[0xee; 32]);

    let frame = Frame { code: MessageCode::INV, body: ids_body };
    let msg = ProtocolMessage::from_frame(&frame).unwrap();
    let back = msg.to_frame();
    assert_eq!(back.code, frame.code);
    assert_eq!(back.body, frame.body);
}
