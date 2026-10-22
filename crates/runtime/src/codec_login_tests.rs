//! Actual managed-login wire shapes over owned synthetic endpoints only.
#![forbid(unsafe_code)]
use super::super::super::codec::{CancelStatus, Presence};
use super::*;
fn take_frame(driver: &mut TransportDriver, session: &mut ProtocolSession) -> ReaderFrame {
    loop {
        match driver.next(session).unwrap() {
            DriverEvent::CancellationRequired => {}
            DriverEvent::Written(Method::LoginCancel) => {}
            DriverEvent::Frame(frame) => return frame,
            _ => panic!("expected managed wire frame"),
        }
    }
}
fn setup() -> (
    ProtocolSession,
    TransportDriver,
    Peer,
    Codec,
    io::CancelHandle,
) {
    let mut s = ProtocolSession::for_login_test(Instant::now()).unwrap();
    let (mut d, peer, cancel) = harness(&s);
    let c = Codec::for_test(&d, &s);
    ready_codec(&mut s, &mut d, &peer, &c);
    (s, d, peer, c, cancel)
}
fn drain(peer: &Peer) {
    while matches!(peer.input.try_read().unwrap(), ReadStep::Data(_)) {}
}
fn start(s: &mut ProtocolSession, d: &mut TransportDriver, c: &Codec) {
    c.start_login(s, d).unwrap();
    written(d, s, Method::LoginStart);
}
fn reply(peer: &Peer, id: u64, value: &str) {
    response(peer, id, value);
}
fn event(
    s: &mut ProtocolSession,
    d: &mut TransportDriver,
    peer: &Peer,
    c: &Codec,
    method: &str,
    params: &str,
) -> Result<CodecEvent, DriverError> {
    assert!(peer
        .output
        .try_write(format!("{{\"method\":\"{method}\",\"params\":{params}}}\n").as_bytes())
        .unwrap());
    let frame = take_frame(d, s);
    c.consume(s, d, frame)
}
const START: &str = r#"{"type":"chatgpt","loginId":"SYNTHETIC_ID","authUrl":"https://synthetic.invalid/PRIVATE_URL"}"#;
const COMPLETE: &str = r#"{"loginId":"SYNTHETIC_ID","success":true}"#;
fn start_reply(
    s: &mut ProtocolSession,
    d: &mut TransportDriver,
    peer: &Peer,
    c: &Codec,
) -> CodecEvent {
    reply(peer, 2, START);
    let frame = take_frame(d, s);
    c.consume(s, d, frame).unwrap()
}
fn cancel(s: &mut ProtocolSession, d: &mut TransportDriver, c: &Codec) {
    assert_eq!(s.cancel(Instant::now()), Err(SessionError::Cancelled));
    c.cancel_login(s, d).unwrap();
}
fn cancel_written(s: &mut ProtocolSession, d: &mut TransportDriver) {
    loop {
        match d.next(s).unwrap() {
            DriverEvent::CancellationRequired => {}
            DriverEvent::Written(Method::LoginCancel) => break,
            _ => panic!("wrong cancellation event"),
        }
    }
}

#[test]
fn managed_fixed_start_and_cancel_bytes_require_written_receipts() {
    let (mut s, mut d, peer, c, _cancel) = setup();
    drain(&peer);
    start(&mut s, &mut d, &c);
    let ReadStep::Data(chunk) = peer.input.try_read().unwrap() else {
        panic!("missing request")
    };
    assert_eq!(
        chunk.bytes.as_slice(),
        b"{\"id\":2,\"method\":\"account/login/start\",\"params\":{\"type\":\"chatgpt\"}}\n"
    );
    let observed = start_reply(&mut s, &mut d, &peer, &c);
    assert!(matches!(observed, CodecEvent::LoginStarted { .. }));
    assert!(!format!("{observed:?}").contains("PRIVATE_URL"));
    let CodecEvent::LoginStarted { auth_url } = observed else {
        unreachable!()
    };
    assert_eq!(auth_url.as_str(), "https://synthetic.invalid/PRIVATE_URL");
    cancel(&mut s, &mut d, &c);
    cancel_written(&mut s, &mut d);
    let ReadStep::Data(chunk) = peer.input.try_read().unwrap() else {
        panic!("missing cancel")
    };
    assert_eq!(chunk.bytes.as_slice(), b"{\"id\":3,\"method\":\"account/login/cancel\",\"params\":{\"loginId\":\"SYNTHETIC_ID\"}}\n");
}

#[test]
fn managed_start_requires_all_schema_fields_and_rejects_other_modes() {
    for payload in [
        "{}",
        "null",
        r#"{"type":"chatgpt","loginId":"SYNTHETIC_ID"}"#,
        r#"{"type":"chatgpt","loginId":null,"authUrl":"x"}"#,
        r#"{"type":"chatgpt","loginId":"x","authUrl":null}"#,
        r#"{"type":"chatgpt","loginId":"x","authUrl":3}"#,
        r#"{"type":"apiKey"}"#,
        r#"{"type":"chatgptDeviceCode","loginId":"x","verificationUrl":"x","userCode":"x"}"#,
        r#"{"type":"chatgptAuthTokens"}"#,
        r#"{"type":"amazonBedrock"}"#,
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        reply(&peer, 2, payload);
        let frame = take_frame(&mut d, &mut s);
        assert!(c.consume(&mut s, &mut d, frame).is_err());
        assert!(d.first_error.is_some());
    }
}

#[test]
fn managed_login_ids_are_bounded_exact_and_cancel_encoding_escapes_them() {
    for id in ["", &"X".repeat(257)] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        reply(
            &peer,
            2,
            &format!("{{\"type\":\"chatgpt\",\"loginId\":\"{id}\",\"authUrl\":\"x\"}}"),
        );
        let frame = take_frame(&mut d, &mut s);
        assert!(c.consume(&mut s, &mut d, frame).is_err());
    }
    for id in ["X".repeat(256), "é".repeat(128)] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        reply(
            &peer,
            2,
            &format!("{{\"type\":\"chatgpt\",\"loginId\":\"{id}\",\"authUrl\":\"x\"}}"),
        );
        let frame = take_frame(&mut d, &mut s);
        assert!(c.consume(&mut s, &mut d, frame).is_ok());
    }
    let (mut s, mut d, peer, c, _cancel) = setup();
    start(&mut s, &mut d, &c);
    drain(&peer);
    reply(
        &peer,
        2,
        r#"{"type":"chatgpt","loginId":"A\"\\\u00e9 ","authUrl":"x"}"#,
    );
    let frame = take_frame(&mut d, &mut s);
    c.consume(&mut s, &mut d, frame).unwrap();
    cancel(&mut s, &mut d, &c);
    cancel_written(&mut s, &mut d);
    let ReadStep::Data(chunk) = peer.input.try_read().unwrap() else {
        panic!("missing escaped cancel")
    };
    assert_eq!(
        std::str::from_utf8(&chunk.bytes).unwrap(),
        "{\"id\":3,\"method\":\"account/login/cancel\",\"params\":{\"loginId\":\"A\\\"\\\\é \"}}\n"
    );
}

#[test]
fn managed_replies_require_original_reader_session_and_sent_numeric_ticket() {
    for id in ["1", "3", "\"2\"", "257", "2.0"] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        assert!(peer
            .output
            .try_write(format!("{{\"id\":{id},\"result\":{START}}}\n").as_bytes())
            .unwrap());
        let frame = take_frame(&mut d, &mut s);
        assert!(c.consume(&mut s, &mut d, frame).is_err());
    }
    let (mut s, mut d, peer, c, _cancel) = setup();
    s.start_login(Instant::now()).unwrap();
    reply(&peer, 2, START);
    let frame = take_frame(&mut d, &mut s);
    assert_eq!(
        c.consume(&mut s, &mut d, frame).unwrap_err().primary,
        SessionError::UnsentResponse
    );
    let (mut s, mut d, _peer, c, _cancel) = setup();
    start(&mut s, &mut d, &c);
    let (mut other, peer, _cancel) = harness(&s);
    reply(&peer, 2, START);
    let frame = take_frame(&mut other, &mut s);
    assert!(c.consume(&mut s, &mut d, frame).is_err());
}

#[test]
fn managed_completion_early_correlation_and_nullable_id_refusals() {
    for params in [
        r#"{"success":true}"#,
        r#"{"success":true,"loginId":null}"#,
        r#"{"success":true,"loginId":""}"#,
        r#"{"loginId":"SYNTHETIC_ID"}"#,
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        assert!(event(&mut s, &mut d, &peer, &c, "account/login/completed", params).is_err());
    }
    let (mut s, mut d, peer, c, _cancel) = setup();
    start(&mut s, &mut d, &c);
    let buffered = event(
        &mut s,
        &mut d,
        &peer,
        &c,
        "account/login/completed",
        COMPLETE,
    )
    .unwrap();
    assert!(matches!(buffered, CodecEvent::LoginCompletionBuffered));
    assert_eq!(s.early_login_notifications(), 1);
    assert_eq!(s.login_report(), None);
    start_reply(&mut s, &mut d, &peer, &c);
    assert_eq!(s.login_report(), Some(LoginCompletion::Success));
}

#[test]
fn managed_all_six_wire_start_completion_cancel_orders_preserve_intent_and_discard_late_url() {
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        for step in order {
            match step {
                0 => {
                    let late = s.phase() == SessionPhase::LoginCancelling;
                    let e = start_reply(&mut s, &mut d, &peer, &c);
                    if late {
                        assert!(matches!(e, CodecEvent::CancellationReady));
                    }
                }
                1 => {
                    event(
                        &mut s,
                        &mut d,
                        &peer,
                        &c,
                        "account/login/completed",
                        COMPLETE,
                    )
                    .unwrap();
                }
                _ => {
                    assert_eq!(s.cancel(Instant::now()), Err(SessionError::Cancelled));
                }
            }
        }
        if s.phase() == SessionPhase::LoginCancelling {
            c.cancel_login(&mut s, &mut d).unwrap();
            cancel_written(&mut s, &mut d);
            reply(&peer, 3, r#"{"status":"canceled"}"#);
            let frame = take_frame(&mut d, &mut s);
            assert!(matches!(
                c.consume(&mut s, &mut d, frame).unwrap(),
                CodecEvent::CancellationAcknowledged {
                    primary: SessionError::Cancelled,
                    ..
                }
            ));
        }
        assert_eq!(s.failure(), Some(SessionError::Cancelled));
        assert_eq!(s.login_report(), None);
    }
}

#[test]
fn managed_wrong_duplicate_and_conflicting_completions_never_replace_a_match() {
    for second in [COMPLETE, r#"{"loginId":"SYNTHETIC_ID","success":false}"#] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        event(
            &mut s,
            &mut d,
            &peer,
            &c,
            "account/login/completed",
            COMPLETE,
        )
        .unwrap();
        assert!(event(&mut s, &mut d, &peer, &c, "account/login/completed", second).is_err());
    }
    let (mut s, mut d, peer, c, _cancel) = setup();
    start(&mut s, &mut d, &c);
    start_reply(&mut s, &mut d, &peer, &c);
    assert!(event(
        &mut s,
        &mut d,
        &peer,
        &c,
        "account/login/completed",
        r#"{"loginId":"synthetic_id","success":true}"#
    )
    .is_err());
}

#[test]
fn managed_completion_error_is_optional_nullable_untrusted_and_never_classified_by_text() {
    for (success, suffix) in [
        (true, ""),
        (true, ",\"error\":null"),
        (true, ",\"error\":\"PRIVATE_UNKNOWN\""),
        (true, ",\"onboardingEntrypoint\":\"life_sciences\""),
        (false, ""),
        (false, ",\"error\":null"),
        (false, ",\"error\":\"PRIVATE_UNKNOWN\""),
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        start_reply(&mut s, &mut d, &peer, &c);
        let e = event(
            &mut s,
            &mut d,
            &peer,
            &c,
            "account/login/completed",
            &format!("{{\"loginId\":\"SYNTHETIC_ID\",\"success\":{success}{suffix}}}"),
        );
        assert_eq!(e.is_ok(), success);
        assert!(!format!("{e:?}").contains("PRIVATE_UNKNOWN"));
        if let Ok(CodecEvent::LoginCompletion {
            outcome,
            error,
            onboarding,
        }) = e
        {
            assert_eq!(outcome, LoginCompletion::Success);
            match suffix {
                "" => assert!(matches!(error, Presence::Missing)),
                ",\"error\":null" => assert!(matches!(error, Presence::Null)),
                ",\"error\":\"PRIVATE_UNKNOWN\"" => assert!(matches!(error, Presence::Present(_))),
                _ => assert!(matches!(onboarding, Presence::Present("life_sciences"))),
            }
        }
        if !success {
            assert_eq!(s.failure(), Some(SessionError::LoginFailed));
        }
    }
}

#[test]
fn managed_cancel_waits_for_id_and_does_not_allocate_duplicate_or_unsent_receipts() {
    let (mut s, mut d, _peer, c, _cancel) = setup();
    assert_eq!(
        c.cancel_login(&mut s, &mut d).unwrap_err().primary,
        SessionError::Sequence
    );
    let (mut s, mut d, peer, c, _cancel) = setup();
    start(&mut s, &mut d, &c);
    cancel(&mut s, &mut d, &c);
    assert_eq!(s.cancellation_state(), CancellationState::WaitingForId);
    assert!(d.write.is_none());
    start_reply(&mut s, &mut d, &peer, &c);
    c.cancel_login(&mut s, &mut d).unwrap();
    assert!(c.cancel_login(&mut s, &mut d).is_err());
}

#[test]
fn managed_cancel_exact_status_is_secondary_acknowledgement_not_exit_or_login_proof() {
    let (mut s, mut d, peer, c, _cancel) = setup();
    start(&mut s, &mut d, &c);
    start_reply(&mut s, &mut d, &peer, &c);
    s.login.as_mut().unwrap().deadline = Some(Instant::now());
    c.cancel_login(&mut s, &mut d).unwrap();
    cancel_written(&mut s, &mut d);
    reply(&peer, 3, r#"{"status":"notFound"}"#);
    let frame = take_frame(&mut d, &mut s);
    assert!(matches!(
        c.consume(&mut s, &mut d, frame).unwrap(),
        CodecEvent::CancellationAcknowledged {
            status: CancelStatus::NotFound,
            primary: SessionError::LoginTimeout
        }
    ));
    for (value, expected) in [
        ("canceled", CancelStatus::Canceled),
        ("notFound", CancelStatus::NotFound),
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        start_reply(&mut s, &mut d, &peer, &c);
        cancel(&mut s, &mut d, &c);
        cancel_written(&mut s, &mut d);
        reply(&peer, 3, &format!("{{\"status\":\"{value}\"}}"));
        let frame = take_frame(&mut d, &mut s);
        let e = c.consume(&mut s, &mut d, frame).unwrap();
        assert!(
            matches!(e, CodecEvent::CancellationAcknowledged { status, primary: SessionError::Cancelled } if status == expected)
        );
        assert_eq!(s.cancellation_state(), CancellationState::Acknowledged);
        assert_eq!(s.login_report(), None);
        assert!(d.first_error.is_some());
        assert!(peer
            .output
            .try_write(b"{\"method\":\"account/updated\",\"params\":{}}\n")
            .is_err());
        assert_eq!(
            c.start_login(&mut s, &mut d).unwrap_err().primary,
            SessionError::Cancelled
        );
        assert_eq!(d.next(&mut s).unwrap_err().primary, SessionError::Cancelled);
    }
    for payload in [
        "{}",
        r#"{"status":null}"#,
        r#"{"status":"cancelled"}"#,
        r#"{"status":"unknown"}"#,
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        start_reply(&mut s, &mut d, &peer, &c);
        cancel(&mut s, &mut d, &c);
        cancel_written(&mut s, &mut d);
        reply(&peer, 3, payload);
        let frame = take_frame(&mut d, &mut s);
        assert!(c.consume(&mut s, &mut d, frame).is_err());
        assert_ne!(s.cancellation_state(), CancellationState::Acknowledged);
    }
}

#[test]
fn managed_remote_errors_keep_cancel_primary_and_fixed_secondary() {
    for cancelling in [false, true] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        let id = if cancelling {
            start_reply(&mut s, &mut d, &peer, &c);
            cancel(&mut s, &mut d, &c);
            cancel_written(&mut s, &mut d);
            3
        } else {
            2
        };
        assert!(peer
            .output
            .try_write(
                format!(
                    "{{\"id\":{id},\"error\":{{\"code\":-1,\"message\":\"PRIVATE_ERROR\"}}}}\n"
                )
                .as_bytes()
            )
            .unwrap());
        let frame = take_frame(&mut d, &mut s);
        let e = c.consume(&mut s, &mut d, frame).unwrap_err();
        assert_eq!(
            e.primary,
            if cancelling {
                SessionError::Cancelled
            } else {
                SessionError::RemoteError
            }
        );
        if cancelling {
            assert_eq!(e.cancellation_error, Some(SessionError::RemoteError));
        }
        assert!(!format!("{e:?}").contains("PRIVATE_ERROR"));
    }
}

#[test]
fn managed_account_signals_share_32_credit_with_early_completion_including_cancel() {
    let (mut s, mut d, peer, c, _cancel) = setup();
    start(&mut s, &mut d, &c);
    event(
        &mut s,
        &mut d,
        &peer,
        &c,
        "account/login/completed",
        COMPLETE,
    )
    .unwrap();
    for _ in 0..31 {
        event(&mut s, &mut d, &peer, &c, "account/updated", "{}").unwrap();
    }
    assert_eq!(s.pending_notifications(), 31);
    assert_eq!(s.early_login_notifications(), 1);
    assert!(event(&mut s, &mut d, &peer, &c, "account/updated", "{}").is_err());
    let (mut s, mut d, peer, c, _cancel) = setup();
    start(&mut s, &mut d, &c);
    start_reply(&mut s, &mut d, &peer, &c);
    cancel(&mut s, &mut d, &c);
    let deadline = s.poll_cancellation(Instant::now()).unwrap();
    for _ in 0..32 {
        event(&mut s, &mut d, &peer, &c, "account/updated", "{}").unwrap();
    }
    assert_eq!(s.poll_cancellation(Instant::now()).unwrap(), deadline);
    assert!(s.take_notification(Instant::now()).is_err());
    assert!(event(&mut s, &mut d, &peer, &c, "account/updated", "{}").is_err());
    assert_eq!(s.failure(), Some(SessionError::Cancelled));
    assert_eq!(
        s.cancellation_error(),
        Some(SessionError::NotificationLimit)
    );
}

#[test]
fn managed_account_updated_preserves_missing_null_and_managed_metadata() {
    for plan in [
        "free",
        "go",
        "plus",
        "pro",
        "prolite",
        "promax",
        "team",
        "self_serve_business_prolite",
        "self_serve_business_usage_based",
        "business",
        "ent26",
        "enterprise_cbp_automation",
        "enterprise_cbp_usage_based",
        "enterprise",
        "edu",
        "edu_plus",
        "edu_pro",
        "unknown",
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        let value = event(
            &mut s,
            &mut d,
            &peer,
            &c,
            "account/updated",
            &format!("{{\"authMode\":\"chatgpt\",\"planType\":\"{plan}\"}}"),
        );
        assert!(value.is_ok());
    }
    for (params, state) in [
        ("{}", 0),
        (r#"{"authMode":null,"planType":null}"#, 1),
        (r#"{"authMode":"chatgpt","planType":"plus"}"#, 2),
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        let CodecEvent::AccountUpdated(observation) =
            event(&mut s, &mut d, &peer, &c, "account/updated", params).unwrap()
        else {
            panic!("wrong event")
        };
        match (state, observation.auth_mode, observation.plan) {
            (0, Presence::Missing, Presence::Missing)
            | (1, Presence::Null, Presence::Null)
            | (2, Presence::Present("chatgpt"), Presence::Present("plus")) => {}
            _ => panic!("presence collapsed"),
        }
    }
    for params in [
        r#"{"authMode":"apikey"}"#,
        r#"{"authMode":"chatgptAuthTokens"}"#,
        r#"{"planType":"invented"}"#,
        "null",
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        assert!(event(&mut s, &mut d, &peer, &c, "account/updated", params).is_err());
    }
}

#[test]
fn managed_emission_timestamp_is_notification_only_and_never_deadline_authority() {
    for stamp in ["0", "-1", "9223372036854775807"] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        let deadline = s.total_deadline;
        assert!(peer
            .output
            .try_write(
                format!(
                    "{{\"method\":\"account/updated\",\"params\":{{}},\"emittedAtMs\":{stamp}}}\n"
                )
                .as_bytes()
            )
            .unwrap());
        let frame = take_frame(&mut d, &mut s);
        c.consume(&mut s, &mut d, frame).unwrap();
        assert_eq!(s.total_deadline, deadline);
    }
    for raw in [
        r#"{"method":"account/updated","params":{},"emittedAtMs":null}"#,
        r#"{"method":"account/updated","params":{},"emittedAtMs":"0"}"#,
        r#"{"id":2,"result":{},"emittedAtMs":0}"#,
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        assert!(peer
            .output
            .try_write(format!("{raw}\n").as_bytes())
            .unwrap());
        let frame = take_frame(&mut d, &mut s);
        assert!(c.consume(&mut s, &mut d, frame).is_err());
    }
}

#[test]
fn managed_expiry_blocked_write_and_late_ack_never_reopen_the_driver() {
    let (mut s, mut d, peer, c, _cancel) = setup();
    start(&mut s, &mut d, &c);
    start_reply(&mut s, &mut d, &peer, &c);
    cancel(&mut s, &mut d, &c);
    cancel_written(&mut s, &mut d);
    reply(&peer, 3, r#"{"status":"canceled"}"#);
    let frame = take_frame(&mut d, &mut s);
    s.login.as_mut().unwrap().cancel_deadline = Some(Instant::now());
    assert_eq!(
        c.consume(&mut s, &mut d, frame).unwrap_err().primary,
        SessionError::Cancelled
    );
    assert_eq!(
        s.cancellation_error(),
        Some(SessionError::CancellationTimeout)
    );
    let (mut s, mut d, peer, c, _cancel) = setup();
    drain(&peer);
    for _ in 0..io::QUEUE_CHUNKS {
        assert!(d.stdin.try_write(SAMPLE).unwrap());
    }
    c.start_login(&mut s, &mut d).unwrap();
    reply(&peer, 2, START);
    let frame = take_frame(&mut d, &mut s);
    assert_eq!(
        c.consume(&mut s, &mut d, frame).unwrap_err().primary,
        SessionError::UnsentResponse
    );
}

#[test]
fn managed_hostile_notifications_are_syntax_checked_bound_and_terminal() {
    let (mut s, mut d, peer, c, _cancel) = setup();
    assert!(peer
        .output
        .try_write(b"{\"method\":\"future/notification\",\"params\":{}}\n")
        .unwrap());
    let frame = take_frame(&mut d, &mut s);
    assert!(matches!(
        c.consume(&mut s, &mut d, frame).unwrap(),
        CodecEvent::IgnoredNotification
    ));
    // Ignored metadata does not leave a pending account signal that blocks login.
    start(&mut s, &mut d, &c);
    for raw in [
        r#"{"method":"account/login/completed","params":{"loginId":"SYNTHETIC_ID","success":true,"suc\u0063ess":false}}"#,
        r#"{"method":"account/login/completed","params":{"loginId":"SYNTHETIC_ID","success":true,"onboardingEntrypoint":"unknown"}}"#,
        r#"{"id":3,"method":"account/login/completed","params":{}}"#,
    ] {
        let (mut s, mut d, peer, c, _cancel) = setup();
        start(&mut s, &mut d, &c);
        assert!(peer
            .output
            .try_write(format!("{raw}\n").as_bytes())
            .unwrap());
        let first = match d.next(&mut s) {
            Ok(DriverEvent::Frame(frame)) => c.consume(&mut s, &mut d, frame).unwrap_err(),
            Err(e) => e,
            _ => panic!("wrong event"),
        };
        assert_eq!(d.next(&mut s).unwrap_err(), first);
    }
}
