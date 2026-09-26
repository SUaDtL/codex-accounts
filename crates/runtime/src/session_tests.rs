//! Synthetic normalized events only; no provider, wire-schema or native receipts.
#![forbid(unsafe_code)]
use super::*;
const INITIALIZATION: Duration = ProtocolSession::INITIALIZATION_BUDGET;
const TOTAL: Duration = ProtocolSession::TOTAL_BUDGET;

fn fresh() -> (ProtocolSession, Instant) {
    let t = Instant::now();
    (ProtocolSession::for_test(t).unwrap(), t)
}
fn ready() -> (ProtocolSession, Instant) {
    let (mut s, t) = fresh();
    let init = s.reserve(Method::Initialize, t).unwrap();
    s.sent(init.id(), t).unwrap();
    s.response(init.id(), ResponseOutcome::Success, t).unwrap();
    s.initialized_sent(t).unwrap();
    (s, t)
}
fn request(s: &mut ProtocolSession, t: Instant, method: Method) -> IssuedRequest {
    let r = s.reserve(method, t).unwrap();
    s.sent(r.id(), t).unwrap();
    r
}
fn stopped(s: &mut ProtocolSession, t: Instant, error: SessionError) {
    assert_eq!(s.phase(), SessionPhase::Stopped);
    assert_eq!(s.failure(), Some(error));
    assert_eq!(s.pending_requests(), 0);
    assert_eq!(s.pending_notifications(), 0);
    assert_eq!(s.poll(t), Err(error));
    assert_eq!(s.finish(t), Err(error));
    assert_eq!(s.cancel(t), Err(error));
    assert_eq!(s.transport_failed(t), Err(error));
    assert_eq!(s.reserve(Method::Initialize, t).unwrap_err(), error);
}

#[test]
fn handshake_requires_sent_request_matched_success_and_sent_notification() {
    let (mut s, t) = fresh();
    assert_eq!(s.phase(), SessionPhase::AwaitInitialize);
    assert_eq!(s.poll(t).unwrap(), t + INITIALIZATION);
    let init = s.reserve(Method::Initialize, t).unwrap();
    assert_eq!(init.method(), Method::Initialize);
    assert_eq!(init.deadline(), t + INITIALIZATION);
    assert_eq!(s.phase(), SessionPhase::AwaitInitializeResponse);
    s.sent(init.id(), t).unwrap();
    s.account_notification(t).unwrap(); // Early signal, not a reply or identity.
    assert_eq!(s.pending_requests(), 1);
    assert_eq!(s.phase(), SessionPhase::AwaitInitializeResponse);
    assert_eq!(
        s.response(init.id(), ResponseOutcome::Success, t),
        Ok(Method::Initialize)
    );
    assert_eq!(s.phase(), SessionPhase::AwaitInitialized);
    s.initialized_sent(t).unwrap();
    assert_eq!(s.phase(), SessionPhase::Ready);
    assert!(s.take_notification(t).unwrap());
    assert!(!s.take_notification(t).unwrap());
    assert_eq!(s.poll(t).unwrap(), t + TOTAL);
}

#[test]
fn account_operations_refuse_at_each_incomplete_handshake_stage() {
    for phase in 0..4 {
        for method in [Method::AccountRead, Method::RateLimitsRead] {
            let (mut s, t) = fresh();
            if phase > 0 {
                let init = s.reserve(Method::Initialize, t).unwrap();
                if phase > 1 {
                    s.sent(init.id(), t).unwrap();
                }
                if phase > 2 {
                    s.response(init.id(), ResponseOutcome::Success, t).unwrap();
                }
            }
            assert_eq!(s.reserve(method, t).unwrap_err(), SessionError::Sequence);
            stopped(&mut s, t, SessionError::Sequence);
        }
    }
}

#[test]
fn initialize_repetition_and_early_or_repeated_initialized_refuse() {
    for phase in 0..4 {
        let (mut s, t) = fresh();
        if phase > 0 {
            let init = request(&mut s, t, Method::Initialize);
            if phase > 1 {
                s.response(init.id(), ResponseOutcome::Success, t).unwrap();
            }
            if phase > 2 {
                s.initialized_sent(t).unwrap();
            }
            assert_eq!(
                s.reserve(Method::Initialize, t).unwrap_err(),
                SessionError::Sequence
            );
        } else {
            assert_eq!(s.initialized_sent(t), Err(SessionError::Sequence));
        }
        stopped(&mut s, t, SessionError::Sequence);
    }
    let (mut s, t) = ready();
    assert_eq!(s.initialized_sent(t), Err(SessionError::Sequence));
}

#[test]
fn non_login_scope_rejects_login_and_notification_as_request() {
    for method in [Method::LoginStart, Method::LoginCancel, Method::Initialized] {
        let (mut s, t) = ready();
        assert_eq!(
            s.reserve(method, t).unwrap_err(),
            SessionError::MethodUnavailable
        );
        stopped(&mut s, t, SessionError::MethodUnavailable);
    }
}

#[test]
fn response_before_send_and_duplicate_send_invalidate_the_session() {
    for duplicate in [false, true] {
        let (mut s, t) = fresh();
        let init = s.reserve(Method::Initialize, t).unwrap();
        let error = if duplicate {
            s.sent(init.id(), t).unwrap();
            assert_eq!(s.sent(init.id(), t), Err(SessionError::DuplicateSend));
            SessionError::DuplicateSend
        } else {
            assert_eq!(
                s.response(init.id(), ResponseOutcome::Success, t),
                Err(SessionError::UnsentResponse)
            );
            SessionError::UnsentResponse
        };
        stopped(&mut s, t, error);
    }
}

#[test]
fn local_ticket_owner_lifetime_prevents_cross_session_reuse() {
    let (mut a, t) = fresh();
    let mut b = ProtocolSession::for_test(t).unwrap();
    let ar = request(&mut a, t, Method::Initialize);
    let br = request(&mut b, t, Method::Initialize);
    assert_eq!(ar.id.sequence, br.id.sequence);
    assert_ne!(ar.id(), br.id());
    assert_eq!(
        a.response(br.id(), ResponseOutcome::Success, t),
        Err(SessionError::Correlation)
    );
    // The new session's own cloned ticket is equal and can be consumed once.
    assert_eq!(
        b.response(&br.id().clone(), ResponseOutcome::Success, t),
        Ok(Method::Initialize)
    );
    stopped(&mut a, t, SessionError::Correlation);
}

#[test]
fn all_response_orders_match_requests_without_advancing_other_tickets() {
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let (mut s, t) = ready();
        let methods = [
            Method::AccountRead,
            Method::RateLimitsRead,
            Method::AccountRead,
        ];
        let tickets: Vec<_> = methods.iter().map(|&m| request(&mut s, t, m)).collect();
        assert_ne!(tickets[0].id(), tickets[2].id());
        for (i, index) in order.into_iter().enumerate() {
            s.account_notification(t).unwrap();
            assert_eq!(
                s.response(tickets[index].id(), ResponseOutcome::Success, t),
                Ok(methods[index])
            );
            assert_eq!(s.pending_requests(), 2 - i);
            assert!(s.take_notification(t).unwrap());
        }
        s.finish(t).unwrap();
        assert_eq!(s.phase(), SessionPhase::Closed);
    }
}

#[test]
fn duplicate_and_never_issued_responses_clear_other_pending_work() {
    let (mut s, t) = ready();
    let first = request(&mut s, t, Method::AccountRead);
    s.response(first.id(), ResponseOutcome::Success, t).unwrap();
    request(&mut s, t, Method::AccountRead);
    assert_eq!(
        s.response(first.id(), ResponseOutcome::Success, t),
        Err(SessionError::Correlation)
    );
    stopped(&mut s, t, SessionError::Correlation);
    let (mut s, t) = ready();
    let forged = RequestId {
        owner: s.owner.clone(),
        sequence: 200,
    };
    assert_eq!(
        s.response(&forged, ResponseOutcome::Error, t),
        Err(SessionError::Correlation)
    );
}

#[test]
fn remote_error_is_neither_auth_rejection_nor_success() {
    for handshake in [true, false] {
        let (mut s, t) = if handshake { fresh() } else { ready() };
        let r = request(
            &mut s,
            t,
            if handshake {
                Method::Initialize
            } else {
                Method::AccountRead
            },
        );
        assert_eq!(
            s.response(r.id(), ResponseOutcome::Error, t),
            Err(SessionError::RemoteError)
        );
        stopped(&mut s, t, SessionError::RemoteError);
    }
}

#[test]
fn pending_request_limit_reclaims_credit_without_reusing_ids() {
    let (mut s, t) = ready();
    let tickets: Vec<_> = (0..MAX_PENDING)
        .map(|_| request(&mut s, t, Method::AccountRead))
        .collect();
    s.response(tickets[3].id(), ResponseOutcome::Success, t)
        .unwrap();
    let new = request(&mut s, t, Method::RateLimitsRead);
    assert!(new.id.sequence > tickets.last().unwrap().id.sequence);
    assert_eq!(s.pending_requests(), MAX_PENDING);
    assert_eq!(
        s.reserve(Method::AccountRead, t).unwrap_err(),
        SessionError::PendingLimit
    );
    stopped(&mut s, t, SessionError::PendingLimit);
}

#[test]
fn completed_requests_do_not_reset_lifetime_issuance_limit() {
    let (mut s, t) = ready(); // Initialization consumed ID 1.
    for sequence in 2..=MAX_ISSUED {
        let r = request(&mut s, t, Method::AccountRead);
        assert_eq!(r.id.sequence, sequence);
        s.response(r.id(), ResponseOutcome::Success, t).unwrap();
    }
    assert_eq!(
        s.reserve(Method::AccountRead, t).unwrap_err(),
        SessionError::IssuedLimit
    );
    stopped(&mut s, t, SessionError::IssuedLimit);
}

#[test]
fn initialization_budget_includes_reservation_send_and_initialized_write() {
    let (mut s, t) = fresh();
    let r = s
        .reserve(Method::Initialize, t + Duration::from_secs(8))
        .unwrap();
    assert_eq!(r.deadline(), t + INITIALIZATION);
    s.sent(r.id(), t + Duration::from_secs(9)).unwrap();
    s.response(
        r.id(),
        ResponseOutcome::Success,
        t + Duration::from_millis(9999),
    )
    .unwrap();
    assert_eq!(
        s.initialized_sent(t + INITIALIZATION),
        Err(SessionError::InitializationTimeout)
    );
    stopped(
        &mut s,
        t + INITIALIZATION,
        SessionError::InitializationTimeout,
    );
    let (mut s, t) = fresh();
    assert_eq!(
        s.reserve(Method::Initialize, t + INITIALIZATION)
            .unwrap_err(),
        SessionError::InitializationTimeout
    );
}

#[test]
fn response_one_nanosecond_before_deadline_passes_at_deadline_refuses() {
    for exact in [false, true] {
        let (mut s, t) = ready();
        let r = request(&mut s, t, Method::AccountRead);
        let now = if exact {
            r.deadline()
        } else {
            r.deadline() - Duration::from_nanos(1)
        };
        if exact {
            assert_eq!(
                s.response(r.id(), ResponseOutcome::Success, now),
                Err(SessionError::RequestTimeout)
            );
            stopped(&mut s, now, SessionError::RequestTimeout);
        } else {
            assert_eq!(
                s.response(r.id(), ResponseOutcome::Success, now),
                Ok(Method::AccountRead)
            );
        }
    }
}

#[test]
fn queued_time_counts_and_sending_does_not_restart_request_budget() {
    let (mut s, t) = ready();
    let r = s
        .reserve(Method::AccountRead, t + Duration::from_secs(2))
        .unwrap();
    s.sent(r.id(), t + Duration::from_secs(16)).unwrap();
    assert_eq!(
        s.poll(t + Duration::from_secs(16)),
        Ok(t + Duration::from_secs(17))
    );
    assert_eq!(
        s.response(
            r.id(),
            ResponseOutcome::Success,
            t + Duration::from_secs(17)
        ),
        Err(SessionError::RequestTimeout)
    );
}

#[test]
fn expired_older_request_blocks_a_timely_reply_to_a_newer_request() {
    let (mut s, t) = ready();
    request(&mut s, t, Method::AccountRead);
    let later = request(&mut s, t + Duration::from_secs(5), Method::RateLimitsRead);
    assert_eq!(s.poll(t + Duration::from_secs(5)), Ok(t + REQUEST));
    assert_eq!(
        s.response(later.id(), ResponseOutcome::Success, t + REQUEST),
        Err(SessionError::RequestTimeout)
    );
    stopped(&mut s, t + REQUEST, SessionError::RequestTimeout);
}

#[test]
fn progress_and_notifications_do_not_extend_the_total_non_login_budget() {
    let (mut s, t) = ready();
    for sec in [0, 10, 20, 29] {
        let now = t + Duration::from_secs(sec);
        let r = request(&mut s, now, Method::AccountRead);
        assert_eq!(r.deadline(), (now + REQUEST).min(t + TOTAL));
        s.response(r.id(), ResponseOutcome::Success, now).unwrap();
        s.account_notification(now).unwrap();
        s.take_notification(now).unwrap();
    }
    assert_eq!(s.poll(t + Duration::from_secs(29)), Ok(t + TOTAL));
    assert_eq!(s.poll(t + TOTAL), Err(SessionError::TotalTimeout));
    stopped(&mut s, t + TOTAL, SessionError::TotalTimeout);
}

#[test]
fn every_event_checks_expiry_before_delivering_progress_or_clean_eof() {
    for event in 0..11 {
        let (mut s, t) = ready();
        let r = request(&mut s, t + Duration::from_secs(20), Method::AccountRead);
        let now = t + TOTAL;
        let result = match event {
            0 => s.reserve(Method::AccountRead, now).map(|_| ()),
            1 => s.sent(r.id(), now),
            2 => s
                .response(r.id(), ResponseOutcome::Success, now)
                .map(|_| ()),
            3 => s.initialized_sent(now),
            4 => s.account_notification(now),
            5 => s.take_notification(now).map(|_| ()),
            6 => s.finish(now),
            7 => s.cancel(now),
            8 => s.transport_failed(now),
            9 => s.invalid_message(now),
            _ => s.unexpected_server_request(now),
        };
        assert_eq!(result, Err(SessionError::TotalTimeout));
        stopped(&mut s, now, SessionError::TotalTimeout);
    }
}

#[test]
fn backwards_clock_refuses_and_forward_jump_expires_without_panicking() {
    let (mut s, t) = ready();
    s.poll(t + Duration::from_secs(2)).unwrap();
    assert_eq!(
        s.poll(t + Duration::from_secs(1)),
        Err(SessionError::ClockRegression)
    );
    stopped(&mut s, t, SessionError::ClockRegression);
    let (mut s, t) = ready();
    s.poll(t).unwrap();
    s.poll(t).unwrap();
    assert_eq!(
        s.poll(t + Duration::from_secs(1000)),
        Err(SessionError::TotalTimeout)
    );
}

#[test]
fn notifications_before_send_refuse_and_thirty_third_pending_signal_refuses() {
    for reserved in [false, true] {
        let (mut s, t) = fresh();
        if reserved {
            s.reserve(Method::Initialize, t).unwrap();
        }
        assert_eq!(s.account_notification(t), Err(SessionError::Sequence));
    }
    let (mut s, t) = fresh();
    request(&mut s, t, Method::Initialize);
    for _ in 0..MAX_NOTIFICATIONS {
        s.account_notification(t).unwrap();
    }
    s.take_notification(t).unwrap();
    s.account_notification(t).unwrap();
    assert_eq!(s.pending_notifications(), MAX_NOTIFICATIONS);
    assert_eq!(s.phase(), SessionPhase::AwaitInitializeResponse);
    assert_eq!(
        s.account_notification(t),
        Err(SessionError::NotificationLimit)
    );
    stopped(&mut s, t, SessionError::NotificationLimit);
}

#[test]
fn truncated_protocol_eof_refuses_at_handshake_pending_reply_and_pending_signal() {
    for step in 0..6 {
        let (mut s, t) = fresh();
        if step > 0 {
            let r = request(&mut s, t, Method::Initialize);
            if step > 1 {
                s.response(r.id(), ResponseOutcome::Success, t).unwrap();
            }
            if step > 2 {
                s.initialized_sent(t).unwrap();
            }
            if step == 3 {
                s.reserve(Method::AccountRead, t).unwrap();
            }
            if step == 4 {
                request(&mut s, t, Method::AccountRead);
            }
            if step == 5 {
                s.account_notification(t).unwrap();
            }
        }
        assert_eq!(s.finish(t), Err(SessionError::IncompleteEof));
        stopped(&mut s, t, SessionError::IncompleteEof);
    }
}

#[test]
fn clean_protocol_close_is_idempotent_and_cannot_be_reopened() {
    let (mut s, t) = ready();
    s.finish(t).unwrap();
    s.finish(t + TOTAL).unwrap();
    assert_eq!(s.phase(), SessionPhase::Closed);
    assert_eq!(s.failure(), None);
    assert_eq!(
        s.reserve(Method::Initialize, t).unwrap_err(),
        SessionError::Closed
    );
    assert_eq!(s.account_notification(t), Err(SessionError::Closed));
    assert_eq!(s.phase(), SessionPhase::Closed);
}

#[test]
fn cancellation_malformed_input_transport_and_privileged_request_preserve_first_error() {
    for error in [
        SessionError::Cancelled,
        SessionError::InvalidMessage,
        SessionError::TransportFailure,
        SessionError::UnexpectedServerRequest,
    ] {
        let (mut s, t) = ready();
        let r = request(&mut s, t, Method::AccountRead);
        s.account_notification(t).unwrap();
        let result = match error {
            SessionError::Cancelled => s.cancel(t),
            SessionError::InvalidMessage => s.invalid_message(t),
            SessionError::TransportFailure => s.transport_failed(t),
            _ => s.unexpected_server_request(t),
        };
        assert_eq!(result, Err(error));
        assert_eq!(s.response(r.id(), ResponseOutcome::Success, t), Err(error));
        stopped(&mut s, t, error);
    }
}

#[test]
fn rejected_json_can_stop_sequencing_without_exposing_payload_as_protocol_data() {
    let (mut s, t) = ready();
    request(&mut s, t, Method::AccountRead);
    let mut decoder = crate::JsonObjectDecoder::default();
    decoder
        .feed(b"{\"SYNTHETIC_SECRET\":1,\"SYNTHETIC_SECRET\":2}\n")
        .unwrap();
    assert_eq!(
        decoder.next_object().unwrap_err(),
        crate::JsonFrameError::DuplicateKey
    );
    // The future decoder/driver owns this error bridge; CA-05C invents no wire IDs.
    assert_eq!(s.invalid_message(t), Err(SessionError::InvalidMessage));
    stopped(&mut s, t, SessionError::InvalidMessage);
}

#[test]
fn bookkeeping_debug_exposes_neither_ticket_values_nor_session_clock() {
    let (mut s, t) = fresh();
    let r = request(&mut s, t, Method::Initialize);
    assert_eq!(format!("{s:?}"), "ProtocolSession([REDACTED])");
    assert_eq!(format!("{r:?}"), "IssuedRequest([REDACTED])");
    assert_eq!(format!("{:?}", r.id()), "RequestId([REDACTED])");
}
