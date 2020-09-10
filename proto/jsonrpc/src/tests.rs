use net3_msg::prelude::*;

use crate::*;

/// Returns true if it's a notification.
fn is_notification(msg: &Message) -> bool {
    msg.id.is_none() && msg.error.is_none() && !is_str_empty(&msg.method) && msg.params.is_some()
}

/// Returns true if it's a method call request.
fn is_call_request(msg: &Message) -> bool {
    msg.id.is_some() && !is_str_empty(&msg.method) && msg.error.is_none() && msg.params.is_some()
}

/// Returns true if it's a successful method call response.
fn is_call_response(msg: &Message) -> bool {
    msg.id.is_some() && msg.result.is_some() && msg.error.is_none()
}

/// Returns true if it's a call response containing error.
fn is_error(msg: &Message) -> bool {
    msg.id.is_some() && msg.result.is_none() && msg.error.is_some()
}

fn count_checks(msg: &Message) -> usize {
    is_error(msg) as usize
        + is_call_request(msg) as usize
        + is_call_response(msg) as usize
        + is_notification(msg) as usize
}

fn parse_error() -> Error {
    Error {
        code: ErrorCode(ErrorKind::ErrorCode(-32700)),
        message: "Parse error".to_owned(),
        data: None,
    }
}

#[test]
fn success_output_serialize() {
    let so = Message {
        result: Params::new(&1).unwrap(),
        id: Id::Num(1),
        ..Default::default()
    };
    assert_eq!(count_checks(&so), 1);
    assert_eq!(so.kind(), MessageKind::Response);

    let serialized = serde_json::to_string(&so).unwrap();
    assert_eq!(serialized, r#"{"jsonrpc":"2.0","id":1,"result":1}"#);
}

#[test]
fn success_output_deserialize() {
    let dso = r#"{"jsonrpc":"2.0","id":1,"result":1}"#;

    let deserialized: Message = serde_json::from_str(dso).unwrap();
    assert_eq!(
        deserialized,
        Message {
            version: Version::V2,
            result: Params::new(&1).unwrap(),
            id: Id::Num(1),
            ..Default::default()
        }
    );
    assert_eq!(count_checks(&deserialized), 1);
    assert_eq!(deserialized.kind(), MessageKind::Response);
}

#[test]
fn success_output_method_deserialize() {
    let dso = r#"{"jsonrpc":"2.0","id":1,"method":"test","result":1}"#;

    let deserialized: Message = serde_json::from_str(dso).unwrap();
    assert_eq!(
        deserialized,
        Message {
            version: Version::V2,
            method: Some("test".to_owned()),
            result: Params::new(&1).unwrap(),
            id: Id::Num(1),
            ..Default::default()
        }
    );
    assert_eq!(count_checks(&deserialized), 1);
    assert_eq!(deserialized.kind(), MessageKind::Response);
}

#[test]
fn failure_output_serialize() {
    let fo = Message {
        error: Some(parse_error()),
        id: Id::Num(1),
        ..Default::default()
    };
    assert_eq!(count_checks(&fo), 1);
    assert_eq!(fo.kind(), MessageKind::ErrorResponse);

    let serialized = serde_json::to_string(&fo).unwrap();
    assert_eq!(
        serialized,
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32700,"message":"Parse error"}}"#
    );
}

#[test]
fn failure_output_serialize_jsonrpc_1() {
    let fo = Message {
        error: Some(parse_error()),
        id: Id::Num(1),
        ..Default::default()
    };

    let serialized = serde_json::to_string(&fo).unwrap();
    assert_eq!(
        serialized,
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32700,"message":"Parse error"}}"#
    );
}

#[test]
fn failure_output_deserialize() {
    let dfo = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32700,"message":"Parse error"}}"#;

    let deserialized: Message = serde_json::from_str(dfo).unwrap();
    assert_eq!(
        deserialized,
        Message {
            version: Version::V2,
            error: Some(parse_error()),
            id: Id::Num(1),
            ..Default::default()
        }
    );
}

#[test]
fn single_response_deserialize() {
    let dsr = r#"{"jsonrpc":"2.0","result":1,"id":1}"#;

    let deserialized: Message = serde_json::from_str(dsr).unwrap();
    assert_eq!(
        deserialized,
        Message {
            version: Version::V2,
            result: Params::new(&1).unwrap(),
            id: Id::Num(1),
            ..Default::default()
        }
    );
}

#[test]
fn batch_response_deserialize() {
    let dbr = r#"[{"jsonrpc":"2.0","result":1,"id":1},{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":1}]"#;

    let deserialized: Vec<Message> = serde_json::from_str(dbr).unwrap();
    assert_eq!(
        deserialized,
        vec![
            Message {
                version: Version::V2,
                result: Params::new(&1).unwrap(),
                id: Id::Num(1),
                ..Default::default()
            },
            Message {
                version: Version::V2,
                error: Some(parse_error()),
                id: Id::Num(1),
                ..Default::default()
            }
        ]
    );
    assert_eq!(count_checks(&deserialized.get(0).unwrap()), 1);
    assert_eq!(deserialized.get(0).unwrap().kind(), MessageKind::Response);
    assert_eq!(count_checks(&deserialized.get(1).unwrap()), 1);
    assert_eq!(
        deserialized.get(1).unwrap().kind(),
        MessageKind::ErrorResponse
    );
}

#[test]
fn handle_incorrect_responses() {
    let dsr = r#"
    {
    	"id": 2,
    	"jsonrpc": "2.0",
    	"result": "0x62d3776be72cc7fa62cad6fe8ed873d9bc7ca2ee576e400d987419a3f21079d5",
    	"error": {
    		"message": "VM Exception while processing transaction: revert",
    		"code": -32000,
    		"data": {}
    	}
    }"#;

    let deserialized: Message = serde_json::from_str(dsr).unwrap();
    assert_eq!(count_checks(&deserialized), 0);
    assert_eq!(deserialized.kind(), MessageKind::Undefined);
}
