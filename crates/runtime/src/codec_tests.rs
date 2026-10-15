//! Exact captured-schema shapes over the actual synthetic transport driver.
#![forbid(unsafe_code)]
use super::super::codec::{AccountPresence, Codec, CodecEvent};
use super::super::codec::{RUNTIME_IMAGE_SHA256, RUNTIME_VERSION, SCHEMA_MANIFEST_SHA256};
use super::*;
#[path = "codec_login_tests.rs"]
mod managed;

const INIT: &str = r#"{"codexHome":"C:\\SYNTHETIC_HOME","platformFamily":"windows","platformOs":"windows","userAgent":"SYNTHETIC_AGENT"}"#;
fn bind() -> (
    ProtocolSession,
    TransportDriver,
    Peer,
    Codec,
    io::CancelHandle,
) {
    let session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (driver, peer, cancel) = harness(&session);
    let codec = Codec::for_test(&driver, &session);
    (session, driver, peer, codec, cancel)
}
fn response(peer: &Peer, id: u64, result: &str) {
    let raw = format!("{{\"id\":{id},\"result\":{result}}}\n");
    assert!(peer.output.try_write(raw.as_bytes()).unwrap());
}
fn ready_codec(
    session: &mut ProtocolSession,
    driver: &mut TransportDriver,
    peer: &Peer,
    codec: &Codec,
) {
    codec.queue(session, driver, Method::Initialize).unwrap();
    written(driver, session, Method::Initialize);
    response(peer, 1, INIT);
    let frame = take_frame(driver, session);
    assert!(matches!(
        codec.consume(session, driver, frame).unwrap(),
        CodecEvent::Initialized(_)
    ));
    codec.initialized(session, driver).unwrap();
    written(driver, session, Method::Initialized);
}

#[test]
fn codec_actual_handshake_and_account_shapes_use_exact_wire_bytes_and_receipts() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    let request = c.queue(&mut s, &mut d, Method::Initialize).unwrap();
    assert_eq!(request.id.sequence, 1);
    assert!(!s.pending[0].sent);
    written(&mut d, &mut s, Method::Initialize);
    let ReadStep::Data(chunk) = peer.input.try_read().unwrap() else {
        panic!("missing fixed request")
    };
    assert_eq!(chunk.bytes.as_slice(), b"{\"id\":1,\"method\":\"initialize\",\"params\":{\"clientInfo\":{\"name\":\"local_codex_switcher\",\"version\":\"0.1.0\"}}}\n");
    response(&peer, 1, INIT);
    let frame = take_frame(&mut d, &mut s);
    let result = c.consume(&mut s, &mut d, frame).unwrap();
    assert!(!format!("{result:?}").contains("SYNTHETIC"));
    let CodecEvent::Initialized(observation) = result else {
        panic!("wrong typed result")
    };
    assert_eq!(observation.codex_home.as_str(), "C:\\SYNTHETIC_HOME");
    assert_eq!(observation.platform_family.as_str(), "windows");
    assert_eq!(observation.platform_os.as_str(), "windows");
    assert_eq!(observation.user_agent.as_str(), "SYNTHETIC_AGENT");
    assert_eq!(s.phase(), SessionPhase::AwaitInitialized);
    c.initialized(&mut s, &mut d).unwrap();
    assert_eq!(s.phase(), SessionPhase::AwaitInitialized);
    written(&mut d, &mut s, Method::Initialized);
    let ReadStep::Data(chunk) = peer.input.try_read().unwrap() else {
        panic!("missing notification")
    };
    assert_eq!(chunk.bytes.as_slice(), b"{\"method\":\"initialized\"}\n");
    c.queue(&mut s, &mut d, Method::AccountRead).unwrap();
    written(&mut d, &mut s, Method::AccountRead);
    let ReadStep::Data(chunk) = peer.input.try_read().unwrap() else {
        panic!("missing account request")
    };
    assert_eq!(
        chunk.bytes.as_slice(),
        b"{\"id\":2,\"method\":\"account/read\",\"params\":{\"refreshToken\":false}}\n"
    );
    response(
        &peer,
        2,
        r#"{"requiresOpenaiAuth":true,"account":{"type":"chatgpt","email":null,"planType":"plus"}}"#,
    );
    let frame = take_frame(&mut d, &mut s);
    let CodecEvent::Account(observation) = c.consume(&mut s, &mut d, frame).unwrap() else {
        panic!("wrong typed result")
    };
    assert!(observation.requires_openai_auth);
    let AccountPresence::Managed(account) = observation.account else {
        panic!("missing managed observation")
    };
    assert!(account.email.is_none());
    assert_eq!(account.plan, "plus");
    assert_eq!(s.pending_requests(), 0);
}

#[test]
fn codec_account_nullable_absent_and_nullable_email_are_not_invented_identity() {
    for (index, result) in [
        r#"{"requiresOpenaiAuth":true}"#,
        r#"{"requiresOpenaiAuth":false,"account":null}"#,
        r#"{"requiresOpenaiAuth":true,"account":{"type":"chatgpt","email":"SYNTHETIC_EMAIL","planType":"unknown"}}"#,
    ].into_iter().enumerate() {
        let (mut s, mut d, peer, c, _cancel) = bind();
        ready_codec(&mut s, &mut d, &peer, &c);
        c.queue(&mut s, &mut d, Method::AccountRead).unwrap();
        written(&mut d, &mut s, Method::AccountRead);
        response(&peer, 2, result);
        let frame = take_frame(&mut d, &mut s);
        let event = c.consume(&mut s, &mut d, frame).unwrap();
        assert!(matches!(event, CodecEvent::Account(_)));
        assert!(!format!("{event:?}").contains("SYNTHETIC"));
        let CodecEvent::Account(observation) = event else { unreachable!() };
        assert_eq!(observation.requires_openai_auth, index != 1);
        match (index, observation.account) {
            (0, AccountPresence::Missing) | (1, AccountPresence::Null) => {}
            (2, AccountPresence::Managed(account)) => assert_eq!(account.email.unwrap().as_str(), "SYNTHETIC_EMAIL"),
            _ => panic!("wire presence was collapsed"),
        }
    }
}

#[test]
fn codec_initialize_required_fields_types_and_absolute_home_are_enforced() {
    for result in [
        "null",
        "[]",
        "{}",
        r#"{"userAgent":"SYNTHETIC"}"#,
        r#"{"codexHome":"relative","platformFamily":"windows","platformOs":"windows","userAgent":"a"}"#,
        r#"{"codexHome":null,"platformFamily":"windows","platformOs":"windows","userAgent":"a"}"#,
        r#"{"codexHome":"C:\\x","platformFamily":false,"platformOs":"windows","userAgent":"a"}"#,
        r#"{"codexHome":"C:\\x","platformFamily":"windows","platformOs":"windows","userAgent":3}"#,
    ] {
        let (mut s, mut d, peer, c, _cancel) = bind();
        c.queue(&mut s, &mut d, Method::Initialize).unwrap();
        written(&mut d, &mut s, Method::Initialize);
        response(&peer, 1, result);
        let frame = take_frame(&mut d, &mut s);
        let error = c.consume(&mut s, &mut d, frame).unwrap_err();
        assert_eq!(error.primary, SessionError::InvalidMessage);
        assert!(d.first_error.is_some());
        assert!(!format!("{error:?}").contains("SYNTHETIC"));
    }
}

#[test]
fn codec_account_schema_and_mode_refusals_close_the_actual_driver() {
    for result in [
        "null",
        "{}",
        r#"{"requiresOpenaiAuth":null}"#,
        r#"{"requiresOpenaiAuth":true,"account":{"type":"chatgpt","planType":"plus"}}"#,
        r#"{"requiresOpenaiAuth":true,"account":{"type":"chatgpt","email":0,"planType":"plus"}}"#,
        r#"{"requiresOpenaiAuth":true,"account":{"type":"chatgpt","email":null,"planType":"invented"}}"#,
        r#"{"requiresOpenaiAuth":true,"account":{"type":"apiKey"}}"#,
        r#"{"requiresOpenaiAuth":true,"account":{"type":"amazonBedrock"}}"#,
        r#"{"requiresOpenaiAuth":true,"account":{"type":"future"}}"#,
    ] {
        let (mut s, mut d, peer, c, _cancel) = bind();
        ready_codec(&mut s, &mut d, &peer, &c);
        c.queue(&mut s, &mut d, Method::AccountRead).unwrap();
        written(&mut d, &mut s, Method::AccountRead);
        response(&peer, 2, result);
        let frame = take_frame(&mut d, &mut s);
        assert!(c.consume(&mut s, &mut d, frame).is_err());
        assert!(d.first_error.is_some());
    }
}

#[test]
fn codec_envelopes_reject_unknown_ids_methods_requests_and_remote_errors() {
    for raw in [
        r#"{"id":99,"result":{}}"#,
        r#"{"id":"1","result":{}}"#,
        r#"{"id":0,"result":{}}"#,
        r#"{"id":257,"result":{}}"#,
        r#"{"id":1.0,"result":{}}"#,
        r#"{"id":true,"result":{}}"#,
        r#"{"id":1,"method":"command/exec","params":{"SYNTHETIC":"SECRET"}}"#,
        r#"{"method":"future/notification","params":{}}"#,
        r#"{"id":1,"result":{},"error":{"code":1,"message":"SYNTHETIC"}}"#,
        r#"{"id":1,"error":{"code":1,"message":"SYNTHETIC"}}"#,
        r#"{"id":1,"error":"SYNTHETIC"}"#,
        r#"{"result":{}}"#,
    ] {
        let (mut s, mut d, peer, c, _cancel) = bind();
        c.queue(&mut s, &mut d, Method::Initialize).unwrap();
        written(&mut d, &mut s, Method::Initialize);
        assert!(peer
            .output
            .try_write(format!("{raw}\n").as_bytes())
            .unwrap());
        let frame = take_frame(&mut d, &mut s);
        let first = c.consume(&mut s, &mut d, frame).unwrap_err();
        assert!(!format!("{first:?}").contains("SYNTHETIC"));
        assert_eq!(d.next(&mut s).unwrap_err(), first);
    }
}

#[test]
fn codec_notifications_and_server_requests_have_distinct_fixed_refusals() {
    for (raw, expected) in [
        (
            r#"{"method":"future/notification","params":{}}"#,
            SessionError::MethodUnavailable,
        ),
        (
            r#"{"id":1,"method":"command/exec","params":{}}"#,
            SessionError::UnexpectedServerRequest,
        ),
    ] {
        let (mut s, mut d, peer, c, _cancel) = bind();
        c.queue(&mut s, &mut d, Method::Initialize).unwrap();
        written(&mut d, &mut s, Method::Initialize);
        assert!(peer
            .output
            .try_write(format!("{raw}\n").as_bytes())
            .unwrap());
        let frame = take_frame(&mut d, &mut s);
        assert_eq!(
            c.consume(&mut s, &mut d, frame).unwrap_err().primary,
            expected
        );
    }
}

#[test]
fn codec_unknown_duplicate_stale_and_out_of_order_replies_use_session_tickets() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    for _ in 0..2 {
        c.queue(&mut s, &mut d, Method::AccountRead).unwrap();
        written(&mut d, &mut s, Method::AccountRead);
    }
    for id in [3, 2] {
        response(&peer, id, r#"{"requiresOpenaiAuth":true}"#);
        let frame = take_frame(&mut d, &mut s);
        assert!(c.consume(&mut s, &mut d, frame).is_ok());
    }
    response(&peer, 3, r#"{"requiresOpenaiAuth":true}"#);
    let frame = take_frame(&mut d, &mut s);
    assert_eq!(
        c.consume(&mut s, &mut d, frame).unwrap_err().primary,
        SessionError::Correlation
    );
}

#[test]
fn codec_cross_reader_and_cross_session_frames_refuse_without_normalization() {
    let (mut s, mut d, _peer, c, _cancel) = bind();
    c.queue(&mut s, &mut d, Method::Initialize).unwrap();
    written(&mut d, &mut s, Method::Initialize);
    let (mut foreign, mut other, peer, _other_codec, _other_cancel) = bind();
    response(&peer, 1, INIT);
    let frame = take_frame(&mut other, &mut foreign);
    assert!(c.consume(&mut s, &mut d, frame).is_err());
    assert!(s.failure().is_some());
    let (mut s, mut d, _peer, c, _cancel) = bind();
    c.queue(&mut s, &mut d, Method::Initialize).unwrap();
    written(&mut d, &mut s, Method::Initialize);
    let (mut other, peer, _cancel2) = harness(&s);
    response(&peer, 1, INIT);
    let frame = take_frame(&mut other, &mut s);
    assert!(c.consume(&mut s, &mut d, frame).is_err());
}

#[test]
fn codec_lifetime_numeric_ids_never_reuse_or_exceed_256() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    for id in 2..=256 {
        while matches!(peer.input.try_read().unwrap(), ReadStep::Data(_)) {}
        let request = c.queue(&mut s, &mut d, Method::AccountRead).unwrap();
        assert_eq!(request.id.sequence, id);
        written(&mut d, &mut s, Method::AccountRead);
        response(&peer, id, r#"{"requiresOpenaiAuth":true}"#);
        let frame = take_frame(&mut d, &mut s);
        assert!(c.consume(&mut s, &mut d, frame).is_ok());
    }
    assert_eq!(
        c.queue(&mut s, &mut d, Method::AccountRead)
            .unwrap_err()
            .primary,
        SessionError::IssuedLimit
    );
}

#[test]
fn codec_unsent_response_and_unimplemented_methods_never_bypass_receipts() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    s.reserve(Method::Initialize, Instant::now()).unwrap();
    response(&peer, 1, INIT);
    let frame = take_frame(&mut d, &mut s);
    assert_eq!(
        c.consume(&mut s, &mut d, frame).unwrap_err().primary,
        SessionError::UnsentResponse
    );
    for method in [
        Method::LoginStart,
        Method::LoginCancel,
        Method::RateLimitsRead,
        Method::Initialized,
    ] {
        let (mut s, mut d, _peer, c, _cancel) = bind();
        assert_eq!(
            c.queue(&mut s, &mut d, method).unwrap_err().primary,
            SessionError::MethodUnavailable
        );
    }
}

#[test]
fn codec_actual_driver_syntax_gate_rejects_escaped_duplicates_and_depth() {
    for raw in [
        r#"{"id":1,"\u0069d":1,"result":{}}"#.to_owned(),
        r#"{"id":1,"result":{"requiresOpenaiAuth":true,"requires\u004fpenaiAuth":false}}"#
            .to_owned(),
        "{\"id\":1,\"result\":".to_owned() + &"[".repeat(65) + "0" + &"]".repeat(65) + "}",
        "{\"id\":1,\"result\":NaN}".to_owned(),
    ] {
        let (mut s, mut d, peer, c, _cancel) = bind();
        c.queue(&mut s, &mut d, Method::Initialize).unwrap();
        written(&mut d, &mut s, Method::Initialize);
        assert!(peer
            .output
            .try_write(format!("{raw}\n").as_bytes())
            .unwrap());
        assert!(d.next(&mut s).is_err());
    }
}

#[test]
fn codec_deadlines_and_pending_limit_remain_absolute() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    c.queue(&mut s, &mut d, Method::Initialize).unwrap();
    written(&mut d, &mut s, Method::Initialize);
    response(&peer, 1, INIT);
    let frame = take_frame(&mut d, &mut s);
    s.initialize_deadline = Instant::now();
    assert_eq!(
        c.consume(&mut s, &mut d, frame).unwrap_err().primary,
        SessionError::InitializationTimeout
    );
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    for _ in 0..8 {
        c.queue(&mut s, &mut d, Method::AccountRead).unwrap();
        written(&mut d, &mut s, Method::AccountRead);
    }
    assert_eq!(
        c.queue(&mut s, &mut d, Method::AccountRead)
            .unwrap_err()
            .primary,
        SessionError::PendingLimit
    );
}

#[test]
fn codec_binding_constants_pin_the_actual_collected_runtime_and_manifest() {
    assert_eq!(RUNTIME_VERSION, "0.162.0-alpha.17.2");
    assert_eq!(
        RUNTIME_IMAGE_SHA256,
        "e98c516f1c8935789aabe9122ee7e05ec4c3efd1ba6acf179242e031c495b99e"
    );
    assert_eq!(
        SCHEMA_MANIFEST_SHA256,
        "8a371a4eef9b1f58797e0704e798fe8c9899c35d4264285f4554d18b4540f7b2"
    );
}

#[test]
fn codec_remote_error_never_retains_or_exports_message_or_data() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    c.queue(&mut s, &mut d, Method::Initialize).unwrap();
    written(&mut d, &mut s, Method::Initialize);
    assert!(peer.output.try_write(b"{\"id\":1,\"error\":{\"code\":-1,\"message\":\"SYNTHETIC_SECRET\",\"data\":{\"SYNTHETIC_SECRET\":[null]}}}\n").unwrap());
    let frame = take_frame(&mut d, &mut s);
    let first = c.consume(&mut s, &mut d, frame).unwrap_err();
    assert_eq!(first.primary, SessionError::RemoteError);
    assert!(!format!("{first:?}").contains("SYNTHETIC"));
    assert_eq!(
        c.queue(&mut s, &mut d, Method::RateLimitsRead).unwrap_err(),
        first
    );
}

#[test]
fn codec_partial_output_never_counts_as_sent_or_authorizes_a_response() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    // Fill the actual bounded input queue so the codec's request remains unsent.
    for _ in 0..io::QUEUE_CHUNKS {
        assert!(d.stdin.try_write(SAMPLE).unwrap());
    }
    c.queue(&mut s, &mut d, Method::Initialize).unwrap();
    response(&peer, 1, INIT);
    let frame = take_frame(&mut d, &mut s);
    assert!(!s.pending[0].sent);
    assert_eq!(
        c.consume(&mut s, &mut d, frame).unwrap_err().primary,
        SessionError::UnsentResponse
    );
    assert!(d.write.is_none());
}

#[test]
fn codec_oversized_fields_and_frames_are_rejected_by_the_real_bounded_driver() {
    for size in [4097, MAX_FRAME_BYTES + 1] {
        let (mut s, mut d, peer, c, _cancel) = bind();
        c.queue(&mut s, &mut d, Method::Initialize).unwrap();
        written(&mut d, &mut s, Method::Initialize);
        let raw = format!("{{\"id\":1,\"result\":{{\"codexHome\":\"C:\\\\SYNTHETIC\",\"platformFamily\":\"windows\",\"platformOs\":\"windows\",\"userAgent\":\"{}\"}}}}\n", "S".repeat(size));
        thread::scope(|scope| {
            let producer = scope.spawn(|| {
                let deadline = Instant::now() + Duration::from_secs(10);
                for chunk in raw.as_bytes().chunks(CHUNK_BYTES) {
                    loop {
                        assert!(Instant::now() < deadline);
                        match peer.output.try_write(chunk) {
                            Ok(true) => break,
                            Ok(false) => thread::yield_now(),
                            Err(_) => return,
                        }
                    }
                }
            });
            match d.next(&mut s) {
                Ok(DriverEvent::Frame(frame)) => {
                    assert_eq!(
                        c.consume(&mut s, &mut d, frame).unwrap_err().primary,
                        SessionError::InvalidMessage
                    );
                }
                Err(error) => assert_eq!(error.primary, SessionError::InvalidMessage),
                _ => panic!("oversized wire data was accepted"),
            }
            producer.join().unwrap();
        });
        assert!(d.first_error.is_some());
    }
}

#[test]
fn codec_invalid_utf8_and_trailing_payload_do_not_reach_a_typed_result() {
    for raw in [
        b"{\"id\":1,\"result\":\"\xff\"}\n".as_slice(),
        b"{\"id\":1,\"result\":{}} false\n",
    ] {
        let (mut s, mut d, peer, c, _cancel) = bind();
        c.queue(&mut s, &mut d, Method::Initialize).unwrap();
        written(&mut d, &mut s, Method::Initialize);
        assert!(peer.output.try_write(raw).unwrap());
        assert_eq!(
            d.next(&mut s).unwrap_err().primary,
            SessionError::InvalidMessage
        );
    }
}
