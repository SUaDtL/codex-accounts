//! Actual rate-limit schema shapes over owned synthetic endpoints, never RPC.
#![forbid(unsafe_code)]
use super::super::super::codec::Presence;
use super::super::super::codec::{ReadObservation, Snapshot};
use super::*;

fn present<T>(value: Presence<T>) -> T {
    match value {
        Presence::Present(value) => value,
        _ => panic!("missing typed observation"),
    }
}
fn raw_event(
    s: &mut ProtocolSession,
    d: &mut TransportDriver,
    peer: &Peer,
    c: &Codec,
    raw: &str,
) -> Result<CodecEvent, DriverError> {
    assert!(raw.len() <= CHUNK_BYTES * io::QUEUE_CHUNKS);
    for chunk in raw.as_bytes().chunks(CHUNK_BYTES) {
        assert!(peer.output.try_write(chunk).unwrap());
    }
    let result = loop {
        match d.next(s) {
            Ok(DriverEvent::Frame(frame)) => break c.consume(s, d, frame),
            Ok(DriverEvent::CancellationRequired | DriverEvent::Written(Method::LoginCancel)) => {}
            Err(error) => break Err(error),
            _ => panic!("unexpected rate driver event"),
        }
    };
    if let Err(error) = result {
        assert_eq!(d.next(s).unwrap_err(), error);
    }
    result
}
fn read(result: &str) -> Result<ReadObservation, DriverError> {
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    c.queue(&mut s, &mut d, Method::RateLimitsRead).unwrap();
    written(&mut d, &mut s, Method::RateLimitsRead);
    let event = raw_event(
        &mut s,
        &mut d,
        &peer,
        &c,
        &format!("{{\"id\":2,\"result\":{result}}}\n"),
    )?;
    assert_eq!(s.pending_requests(), 0);
    match event {
        CodecEvent::RateLimits(value) => Ok(value),
        _ => panic!("wrong rate event"),
    }
}
fn snapshot(value: &str) -> Result<Snapshot, DriverError> {
    read(&format!("{{\"rateLimits\":{value}}}")).map(|v| v.rate_limits)
}
fn updated(
    s: &mut ProtocolSession,
    d: &mut TransportDriver,
    peer: &Peer,
    c: &Codec,
    params: &str,
) -> Result<CodecEvent, DriverError> {
    raw_event(
        s,
        d,
        peer,
        c,
        &format!("{{\"method\":\"account/rateLimits/updated\",\"params\":{params}}}\n"),
    )
}

#[test]
fn rates_fixed_request_omits_experiment_capabilities_and_reset_operations() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    while matches!(peer.input.try_read().unwrap(), ReadStep::Data(_)) {}
    c.queue(&mut s, &mut d, Method::RateLimitsRead).unwrap();
    written(&mut d, &mut s, Method::RateLimitsRead);
    let ReadStep::Data(chunk) = peer.input.try_read().unwrap() else {
        panic!("missing fixed rate request")
    };
    assert_eq!(chunk.bytes.as_slice(), b"{\"id\":2,\"method\":\"account/rateLimits/read\",\"params\":{\"excludeResetCreditDetails\":true}}\n");
}

#[test]
fn rates_required_snapshot_and_optional_missing_null_values_do_not_invent_defaults() {
    // Production projection retains absence and the documented numeric metadata,
    // without carrying the private account/bucket/name fields to consumers.
    use crate::{Observed, RateLimitSummary, RateLimitWindow};
    assert_eq!(
        RateLimitSummary::from(snapshot(r#"{}"#).unwrap()),
        RateLimitSummary {
            primary: Observed::Missing,
            secondary: Observed::Missing,
            plan: Observed::Missing
        }
    );
    let projected = RateLimitSummary::from(snapshot(r#"{"primary":{"usedPercent":23,"windowDurationMins":300,"resetsAt":123456},"secondary":null,"planType":"plus","limitId":"PRIVATE_ID","limitName":"PRIVATE_NAME"}"#).unwrap());
    assert_eq!(
        projected.primary,
        Observed::Value(RateLimitWindow {
            used_percent: 23,
            duration_mins: Observed::Value(300),
            resets_at: Observed::Value(123456)
        })
    );
    assert_eq!(projected.secondary, Observed::Null);
    assert_eq!(projected.plan, Observed::Value("plus"));
    assert!(!format!("{projected:?}").contains("PRIVATE"));
    let value = read(r#"{"rateLimits":{}}"#).unwrap();
    assert!(matches!(value.account_id, Presence::Missing));
    assert!(matches!(value.ordinary_allowed, Presence::Missing));
    assert!(matches!(value.buckets, Presence::Missing));
    assert!(matches!(value.reset_credits, Presence::Missing));
    let s = value.rate_limits;
    assert!(matches!(s.credits, Presence::Missing));
    assert!(matches!(s.individual, Presence::Missing));
    assert!(matches!(s.limit_id, Presence::Missing));
    assert!(matches!(s.limit_name, Presence::Missing));
    assert!(matches!(s.model, Presence::Missing));
    assert!(matches!(s.plan, Presence::Missing));
    assert!(matches!(s.primary, Presence::Missing));
    assert!(matches!(s.secondary, Presence::Missing));
    assert!(matches!(s.reached, Presence::Missing));
    assert!(matches!(s.spend_reached, Presence::Missing));
    let value = read(r#"{"rateLimits":{"credits":null,"individualLimit":null,"limitId":null,"limitName":null,"normalModelSlug":null,"planType":null,"primary":null,"secondary":null,"rateLimitReachedType":null,"spendControlReached":null},"accountId":null,"ordinaryUsageAllowed":null,"rateLimitsByLimitId":null,"rateLimitResetCredits":null}"#).unwrap();
    assert!(matches!(value.account_id, Presence::Null));
    assert!(matches!(value.ordinary_allowed, Presence::Null));
    assert!(matches!(value.buckets, Presence::Null));
    assert!(matches!(value.reset_credits, Presence::Null));
    let s = value.rate_limits;
    assert!(matches!(s.credits, Presence::Null));
    assert!(matches!(s.individual, Presence::Null));
    assert!(matches!(s.limit_id, Presence::Null));
    assert!(matches!(s.limit_name, Presence::Null));
    assert!(matches!(s.model, Presence::Null));
    assert!(matches!(s.plan, Presence::Null));
    assert!(matches!(s.primary, Presence::Null));
    assert!(matches!(s.secondary, Presence::Null));
    assert!(matches!(s.reached, Presence::Null));
    assert!(matches!(s.spend_reached, Presence::Null));
    for raw in [
        "null",
        "[]",
        "{}",
        r#"{"rateLimits":null}"#,
        r#"{"rateLimits":false}"#,
        r#"{"rateLimits":{},"unknown":0}"#,
        r#"{"rateLimits":{},"ordinaryUsageAllowed":0}"#,
    ] {
        assert!(read(raw).is_err());
    }
}

#[test]
fn rates_complete_typed_fields_preserve_signed_widths_exact_strings_and_false() {
    let v=read(r#"{"rateLimits":{"credits":{"hasCredits":false,"unlimited":true,"balance":"PRIVATE_BALANCE"},"individualLimit":{"limit":"001.00","used":"000.50","remainingPercent":-2147483648,"resetsAt":9223372036854775807},"limitId":"PRIVATE_ID","limitName":"PRIVATE_NAME","normalModelSlug":"PRIVATE_MODEL","planType":"plus","primary":{"usedPercent":-2147483648,"resetsAt":-9223372036854775808,"windowDurationMins":-1},"secondary":{"usedPercent":2147483647,"resetsAt":null},"rateLimitReachedType":"workspace_owner_usage_limit_reached","spendControlReached":false},"accountId":"PRIVATE_ACCOUNT","ordinaryUsageAllowed":false,"rateLimitsByLimitId":{},"rateLimitResetCredits":{"availableCount":-9223372036854775808,"credits":[{"grantedAt":-9223372036854775808,"id":"PRIVATE_CREDIT","resetType":"unknown","status":"unknown","description":"PRIVATE_DESCRIPTION","expiresAt":9223372036854775807,"title":null}]}}"#).unwrap();
    assert!(!format!("{v:?}").contains("PRIVATE"));
    assert_eq!(present(v.account_id).0.as_str(), "PRIVATE_ACCOUNT");
    assert!(!present(v.ordinary_allowed));
    assert!(present(v.buckets).0.is_empty());
    let r = present(v.reset_credits);
    assert_eq!(r.available_count, i64::MIN);
    let mut rows = present(r.credits).0;
    assert_eq!(rows.len(), 1);
    let row = rows.pop().unwrap();
    assert_eq!(row.granted_at, i64::MIN);
    assert_eq!(row.id.0.as_str(), "PRIVATE_CREDIT");
    assert_eq!(row.reset_type.0, "unknown");
    assert_eq!(row.status.0, "unknown");
    assert_eq!(present(row.description).as_str(), "PRIVATE_DESCRIPTION");
    assert_eq!(present(row.expires_at), i64::MAX);
    assert!(matches!(row.title, Presence::Null));
    let s = v.rate_limits;
    assert!(!format!("{s:?}").contains("PRIVATE"));
    let credits = present(s.credits);
    assert!(!credits.has_credits);
    assert!(credits.unlimited);
    assert_eq!(present(credits.balance).as_str(), "PRIVATE_BALANCE");
    let spend = present(s.individual);
    assert_eq!(spend.limit.as_str(), "001.00");
    assert_eq!(spend.used.as_str(), "000.50");
    assert_eq!(spend.remaining_percent, i32::MIN);
    assert_eq!(spend.resets_at, i64::MAX);
    assert_eq!(present(s.limit_id).0.as_str(), "PRIVATE_ID");
    assert_eq!(present(s.limit_name).as_str(), "PRIVATE_NAME");
    assert_eq!(present(s.model).as_str(), "PRIVATE_MODEL");
    assert_eq!(present(s.plan).0, "plus");
    assert_eq!(present(s.reached).0, "workspace_owner_usage_limit_reached");
    assert!(!present(s.spend_reached));
    let primary = present(s.primary);
    assert_eq!(primary.used_percent, i32::MIN);
    assert_eq!(present(primary.resets_at), i64::MIN);
    assert_eq!(present(primary.duration_mins), -1);
    let secondary = present(s.secondary);
    assert_eq!(secondary.used_percent, i32::MAX);
    assert!(matches!(secondary.resets_at, Presence::Null));
    assert!(matches!(secondary.duration_mins, Presence::Missing));
}

#[test]
fn rates_integer_types_required_nested_fields_and_overflow_refuse() {
    for value in [
        r#"{"primary":{}}"#,
        r#"{"primary":{"usedPercent":null}}"#,
        r#"{"primary":{"usedPercent":1.0}}"#,
        r#"{"primary":{"usedPercent":"1"}}"#,
        r#"{"primary":{"usedPercent":2147483648}}"#,
        r#"{"primary":{"usedPercent":-2147483649}}"#,
        r#"{"primary":{"usedPercent":0,"resetsAt":9223372036854775808}}"#,
        r#"{"primary":{"usedPercent":0,"windowDurationMins":false}}"#,
        r#"{"credits":{"hasCredits":false}}"#,
        r#"{"credits":{"hasCredits":null,"unlimited":false}}"#,
        r#"{"credits":{"hasCredits":false,"unlimited":false,"balance":0}}"#,
        r#"{"individualLimit":{"limit":"1","used":"2","remainingPercent":0}}"#,
        r#"{"individualLimit":{"limit":1,"used":"2","remainingPercent":0,"resetsAt":0}}"#,
        r#"{"individualLimit":{"limit":"1","used":"2","remainingPercent":0,"resetsAt":null}}"#,
    ] {
        assert!(
            snapshot(value).is_err(),
            "accepted unsupported nested shape"
        );
    }
    for value in [
        r#"{"rateLimits":{},"accountId":1}"#,
        r#"{"rateLimits":{},"rateLimitResetCredits":{}}"#,
        r#"{"rateLimits":{},"rateLimitResetCredits":{"availableCount":null}}"#,
        r#"{"rateLimits":{},"rateLimitResetCredits":{"availableCount":9223372036854775808}}"#,
    ] {
        assert!(read(value).is_err());
    }
    for percent in [-1000, 0, 101, 10000] {
        assert_eq!(
            present(
                snapshot(&format!("{{\"primary\":{{\"usedPercent\":{percent}}}}}"))
                    .unwrap()
                    .primary
            )
            .used_percent,
            percent
        );
    }
}

#[test]
fn rates_plan_and_reached_labels_use_only_the_actual_schema() {
    for label in [
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
        assert_eq!(
            present(
                snapshot(&format!("{{\"planType\":\"{label}\"}}"))
                    .unwrap()
                    .plan
            )
            .0,
            label
        );
    }
    for label in [
        "rate_limit_reached",
        "workspace_owner_credits_depleted",
        "workspace_member_credits_depleted",
        "workspace_owner_usage_limit_reached",
        "workspace_member_usage_limit_reached",
    ] {
        assert_eq!(
            present(
                snapshot(&format!("{{\"rateLimitReachedType\":\"{label}\"}}"))
                    .unwrap()
                    .reached
            )
            .0,
            label
        );
    }
    for value in [
        r#"{"planType":"future"}"#,
        r#"{"rateLimitReachedType":"rateLimitReached"}"#,
        r#"{"spendControlReached":0}"#,
    ] {
        assert!(snapshot(value).is_err());
    }
}

#[test]
fn rates_bucket_maps_are_bounded_preserve_empty_and_do_not_merge_views() {
    let v=read(r#"{"rateLimits":{"planType":"plus"},"rateLimitsByLimitId":{"synthetic":{"planType":"free"}}}"#).unwrap();
    assert_eq!(present(v.rate_limits.plan).0, "plus");
    let buckets = present(v.buckets).0;
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0].0 .0.as_str(), "synthetic");
    let mut buckets = buckets;
    assert_eq!(present(buckets.pop().unwrap().1.plan).0, "free");
    for count in [64, 65] {
        let entries = (0..count)
            .map(|i| format!("\"SYNTHETIC_{i}\":{{}}"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            read(&format!(
                "{{\"rateLimits\":{{}},\"rateLimitsByLimitId\":{{{entries}}}}}"
            ))
            .is_ok(),
            count == 64
        );
    }
    for raw in [
        r#"{"rateLimits":{},"rateLimitsByLimitId":[]}"#,
        r#"{"rateLimits":{},"rateLimitsByLimitId":{"a":null}}"#,
        r#"{"rateLimits":{},"rateLimitsByLimitId":{"a":{},"\u0061":{}}}"#,
    ] {
        assert!(read(raw).is_err());
    }
}

#[test]
fn rates_credit_rows_are_bounded_optional_details_and_never_action_tickets() {
    for suffix in ["", ",\"credits\":null", ",\"credits\":[]"] {
        let v=present(read(&format!("{{\"rateLimits\":{{}},\"rateLimitResetCredits\":{{\"availableCount\":99{suffix}}}}}")).unwrap().reset_credits);
        assert_eq!(v.available_count, 99);
        match suffix {
            "" => assert!(matches!(v.credits, Presence::Missing)),
            ",\"credits\":null" => assert!(matches!(v.credits, Presence::Null)),
            _ => assert!(present(v.credits).0.is_empty()),
        }
    }
    for status in ["available", "redeeming", "redeemed", "unknown"] {
        for kind in ["codexRateLimits", "unknown"] {
            assert!(read(&format!("{{\"rateLimits\":{{}},\"rateLimitResetCredits\":{{\"availableCount\":3,\"credits\":[{{\"grantedAt\":0,\"id\":\"SYNTHETIC\",\"resetType\":\"{kind}\",\"status\":\"{status}\"}}]}}}}" )).is_ok());
        }
    }
    let row =
        r#"{"grantedAt":0,"id":"SYNTHETIC","resetType":"codexRateLimits","status":"available"}"#;
    for count in [64, 65] {
        let rows = vec![row; count].join(",");
        assert_eq!(read(&format!("{{\"rateLimits\":{{}},\"rateLimitResetCredits\":{{\"availableCount\":100,\"credits\":[{rows}]}}}}")).is_ok(),count==64);
    }
    for row in [
        "{}",
        r#"{"grantedAt":0,"id":null,"resetType":"unknown","status":"unknown"}"#,
        r#"{"grantedAt":0,"id":"x","resetType":"reset","status":"unknown"}"#,
        r#"{"grantedAt":0,"id":"x","resetType":"unknown","status":"future"}"#,
    ] {
        assert!(read(&format!("{{\"rateLimits\":{{}},\"rateLimitResetCredits\":{{\"availableCount\":0,\"credits\":[{row}]}}}}")).is_err());
    }
}

#[test]
fn rates_ignored_upsell_is_still_strictly_validated_and_never_retained() {
    for upsell in [
        "null",
        "false",
        "3",
        r#""PRIVATE_BANNER""#,
        r#"["PRIVATE_URL"]"#,
        r#"{"url":"https://synthetic.invalid/PRIVATE_URL"}"#,
    ] {
        let value = read(&format!(
            "{{\"rateLimits\":{{}},\"rateLimitUpsell\":{upsell}}}"
        ))
        .unwrap();
        assert!(!format!("{value:?}").contains("PRIVATE"));
    }
    for upsell in [
        r#"{"a":1,"\u0061":2}"#.to_owned(),
        "[".repeat(65) + "0" + &"]".repeat(65),
    ] {
        assert!(read(&format!(
            "{{\"rateLimits\":{{}},\"rateLimitUpsell\":{upsell}}}"
        ))
        .is_err());
    }
}

#[test]
fn rates_decoded_text_and_map_key_byte_bounds_reject_nul_without_normalizing() {
    for size in [256, 257] {
        let key = "é".repeat(size / 2) + if size % 2 == 1 { "x" } else { "" };
        assert_eq!(
            read(&format!(
                "{{\"rateLimits\":{{}},\"rateLimitsByLimitId\":{{\"{key}\":{{}}}}}}"
            ))
            .is_ok(),
            size == 256
        );
        assert_eq!(
            read(&format!("{{\"rateLimits\":{{}},\"accountId\":\"{key}\"}}")).is_ok(),
            size == 256
        );
    }
    for size in [4096, 4097] {
        assert_eq!(
            snapshot(&format!("{{\"limitName\":\"{}\"}}", "x".repeat(size))).is_ok(),
            size == 4096
        );
    }
    for value in [
        r#"{"limitName":"PRIVATE\u0000"}"#,
        r#"{"limitId":"x\u0000"}"#,
    ] {
        assert!(snapshot(value).is_err());
    }
}

#[test]
fn rates_sparse_updates_preserve_presence_and_false_without_cache_or_clock_changes() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    let deadline = s.total_deadline;
    for (params, expected) in [
        (r#"{"rateLimits":{"spendControlReached":true}}"#, 0),
        (r#"{"rateLimits":{"spendControlReached":null}}"#, 1),
        (r#"{"rateLimits":{}}"#, 2),
        (r#"{"rateLimits":{"spendControlReached":false}}"#, 3),
    ] {
        let CodecEvent::RateLimitsUpdated(value) =
            updated(&mut s, &mut d, &peer, &c, params).unwrap()
        else {
            panic!("wrong delta event")
        };
        match (expected, value.spend_reached) {
            (0, Presence::Present(true))
            | (1, Presence::Null)
            | (2, Presence::Missing)
            | (3, Presence::Present(false)) => {}
            _ => panic!("delta presence changed"),
        }
        assert_eq!(s.total_deadline, deadline);
        assert_eq!(s.phase(), SessionPhase::Ready);
    }
    assert_eq!(s.pending_notifications(), 4);
    assert!(s.take_notification(Instant::now()).unwrap());
    assert_eq!(s.pending_notifications(), 3);
    let event=raw_event(&mut s,&mut d,&peer,&c,r#"{"method":"account/rateLimits/updated","params":{"rateLimits":{"primary":{"usedPercent":-1}}},"emittedAtMs":9223372036854775807}
"#).unwrap();
    let CodecEvent::RateLimitsUpdated(value) = event else {
        panic!("wrong delta event")
    };
    assert_eq!(present(value.primary).used_percent, -1);
    assert_eq!(s.total_deadline, deadline);
}

#[test]
fn rates_updates_account_signals_and_early_login_completions_share_one_credit_bound() {
    let mut s = ProtocolSession::for_login_test(Instant::now()).unwrap();
    let (mut d, peer, _cancel) = harness(&s);
    let c = Codec::for_test(&d, &s);
    ready_codec(&mut s, &mut d, &peer, &c);
    c.start_login(&mut s, &mut d).unwrap();
    written(&mut d, &mut s, Method::LoginStart);
    raw_event(&mut s,&mut d,&peer,&c,"{\"method\":\"account/login/completed\",\"params\":{\"loginId\":\"SYNTHETIC\",\"success\":true}}\n").unwrap();
    raw_event(
        &mut s,
        &mut d,
        &peer,
        &c,
        "{\"method\":\"account/updated\",\"params\":{}}\n",
    )
    .unwrap();
    for _ in 0..30 {
        updated(&mut s, &mut d, &peer, &c, r#"{"rateLimits":{}}"#).unwrap();
    }
    assert_eq!(s.pending_notifications(), 31);
    assert_eq!(s.early_login_notifications(), 1);
    assert!(s.take_notification(Instant::now()).unwrap());
    updated(&mut s, &mut d, &peer, &c, r#"{"rateLimits":{}}"#).unwrap();
    assert_eq!(
        updated(&mut s, &mut d, &peer, &c, r#"{"rateLimits":{}}"#)
            .unwrap_err()
            .primary,
        SessionError::NotificationLimit
    );
}

#[test]
fn rates_cancellation_deltas_keep_primary_phase_deadline_and_never_acknowledge() {
    let mut s = ProtocolSession::for_login_test(Instant::now()).unwrap();
    let (mut d, peer, _cancel) = harness(&s);
    let c = Codec::for_test(&d, &s);
    ready_codec(&mut s, &mut d, &peer, &c);
    c.start_login(&mut s, &mut d).unwrap();
    written(&mut d, &mut s, Method::LoginStart);
    raw_event(&mut s,&mut d,&peer,&c,"{\"id\":2,\"result\":{\"type\":\"chatgpt\",\"loginId\":\"SYNTHETIC\",\"authUrl\":\"https://synthetic.invalid\"}}\n").unwrap();
    assert_eq!(s.cancel(Instant::now()), Err(SessionError::Cancelled));
    let deadline = s.poll_cancellation(Instant::now()).unwrap();
    for _ in 0..32 {
        updated(
            &mut s,
            &mut d,
            &peer,
            &c,
            r#"{"rateLimits":{"spendControlReached":false}}"#,
        )
        .unwrap();
    }
    assert_eq!(s.phase(), SessionPhase::LoginCancelling);
    assert_eq!(s.cancellation_state(), CancellationState::Ready);
    assert_eq!(s.failure(), Some(SessionError::Cancelled));
    assert_eq!(s.poll_cancellation(Instant::now()).unwrap(), deadline);
    assert_eq!(s.login_report(), None);
    assert!(s.take_notification(Instant::now()).is_err());
    let first = updated(&mut s, &mut d, &peer, &c, r#"{"rateLimits":{}}"#).unwrap_err();
    assert_eq!(first.primary, SessionError::Cancelled);
    assert_eq!(
        first.cancellation_error,
        Some(SessionError::NotificationLimit)
    );
}

#[test]
fn rates_replies_still_require_owned_written_correlated_tickets_and_allow_reordering() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    for _ in 0..2 {
        c.queue(&mut s, &mut d, Method::RateLimitsRead).unwrap();
        written(&mut d, &mut s, Method::RateLimitsRead);
    }
    for id in [3, 2] {
        assert!(matches!(
            raw_event(
                &mut s,
                &mut d,
                &peer,
                &c,
                &format!("{{\"id\":{id},\"result\":{{\"rateLimits\":{{}}}}}}\n")
            )
            .unwrap(),
            CodecEvent::RateLimits(_)
        ));
    }
    assert_eq!(
        raw_event(
            &mut s,
            &mut d,
            &peer,
            &c,
            "{\"id\":2,\"result\":{\"rateLimits\":{}}}\n"
        )
        .unwrap_err()
        .primary,
        SessionError::Correlation
    );
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    s.reserve(Method::RateLimitsRead, Instant::now()).unwrap();
    assert_eq!(
        raw_event(
            &mut s,
            &mut d,
            &peer,
            &c,
            "{\"id\":2,\"result\":{\"rateLimits\":{}}}\n"
        )
        .unwrap_err()
        .primary,
        SessionError::UnsentResponse
    );
    let (mut s, mut d, _peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &_peer, &c);
    c.queue(&mut s, &mut d, Method::RateLimitsRead).unwrap();
    written(&mut d, &mut s, Method::RateLimitsRead);
    let (mut other, peer, _other_cancel) = harness(&s);
    for chunk in b"{\"id\":2,\"result\":{\"rateLimits\":{}}}\n".chunks(CHUNK_BYTES) {
        assert!(peer.output.try_write(chunk).unwrap());
    }
    let frame = take_frame(&mut other, &mut s);
    assert!(c.consume(&mut s, &mut d, frame).is_err());
}

#[test]
fn rates_update_envelopes_types_unknown_methods_and_remote_errors_are_terminal() {
    for raw in [
        "{\"method\":\"account/rateLimits/updated\",\"params\":{}}\n",
        "{\"method\":\"account/rateLimits/updated\",\"params\":null}\n",
        "{\"method\":\"account/rateLimits/updated\",\"params\":{\"rateLimits\":null}}\n",
        "{\"method\":\"account/rateLimits/updated\",\"params\":{\"rateLimits\":{}},\"emittedAtMs\":1.0}\n",
        "{\"method\":\"account/rateLimits/updated\",\"params\":{\"rateLimits\":{}},\"emittedAtMs\":9223372036854775808}\n",
        "{\"id\":2,\"method\":\"account/rateLimits/updated\",\"params\":{\"rateLimits\":{}}}\n",
        "{\"method\":\"account/rateLimits/updated\",\"params\":{\"rateLimits\":{\"credits\":{\"hasCredits\":true,\"has\\u0043redits\":false,\"unlimited\":false}}}}\n",
    ] {let(mut s,mut d,peer,c,_cancel)=bind();ready_codec(&mut s,&mut d,&peer,&c);assert!(raw_event(&mut s,&mut d,&peer,&c,raw).is_err());}
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    c.queue(&mut s, &mut d, Method::RateLimitsRead).unwrap();
    written(&mut d, &mut s, Method::RateLimitsRead);
    let error=raw_event(&mut s,&mut d,&peer,&c,"{\"id\":2,\"error\":{\"code\":-1,\"message\":\"PRIVATE_ERROR\",\"data\":{\"PRIVATE\":null}}}\n").unwrap_err();
    assert_eq!(error.primary, SessionError::RemoteError);
    assert!(!format!("{error:?}").contains("PRIVATE"));
}

#[test]
fn rates_updates_before_handshake_or_after_expiry_never_grant_availability() {
    let (mut s, mut d, peer, c, _cancel) = bind();
    assert_eq!(
        updated(&mut s, &mut d, &peer, &c, r#"{"rateLimits":{}}"#)
            .unwrap_err()
            .primary,
        SessionError::Sequence
    );
    let (mut s, mut d, peer, c, _cancel) = bind();
    ready_codec(&mut s, &mut d, &peer, &c);
    s.total_deadline = Instant::now();
    assert_eq!(
        updated(
            &mut s,
            &mut d,
            &peer,
            &c,
            r#"{"rateLimits":{"spendControlReached":false}}"#
        )
        .unwrap_err()
        .primary,
        SessionError::TotalTimeout
    );
}
