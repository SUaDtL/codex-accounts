//! Deterministic synthetic normalized events. No runtime, filesystem or credentials.
#![forbid(unsafe_code)]
use super::*;
fn fresh() -> (ProtocolSession, Instant) {
    let now = Instant::now();
    (ProtocolSession::for_login_test(now).unwrap(), now)
}
fn handshake(s: &mut ProtocolSession, now: Instant) {
    let r = s.reserve(Method::Initialize, now).unwrap();
    s.sent(r.id(), now).unwrap();
    s.response(r.id(), ResponseOutcome::Success, now).unwrap();
    s.initialized_sent(now).unwrap();
}
fn ready() -> (ProtocolSession, Instant) {
    let (mut s, t) = fresh();
    handshake(&mut s, t);
    (s, t)
}
fn starting() -> (ProtocolSession, Instant, IssuedRequest, LoginId) {
    let (mut s, t) = ready();
    let id = s.login_id_for_test(b"SYNTHETIC_LOGIN_A").unwrap();
    let r = s.start_login(t).unwrap();
    s.sent(r.id(), t).unwrap();
    (s, t, r, id)
}
fn waiting() -> (ProtocolSession, Instant, LoginId) {
    let (mut s, t, r, id) = starting();
    s.login_start_response(r.id(), &id, t).unwrap();
    (s, t, id)
}
fn assert_stopped(s: &mut ProtocolSession, t: Instant, error: SessionError) {
    assert_eq!(s.phase(), SessionPhase::Stopped);
    assert_eq!(s.failure(), Some(error));
    assert_eq!(s.pending_requests(), 0);
    assert_eq!(s.pending_notifications(), 0);
    assert_eq!(s.login_report(), None);
    assert_eq!(s.poll(t), Err(error));
    assert_eq!(s.start_login(t).unwrap_err(), error);
}
#[test]
fn both_completion_orders_require_the_matching_start_response() {
    for early in [true, false] {
        let (mut s, t, r, id) = starting();
        if early {
            s.login_completed(&id, LoginCompletion::Success, t).unwrap();
            assert_eq!(s.login_report(), None);
        }
        s.login_start_response(r.id(), &id, t).unwrap();
        if !early {
            assert_eq!(s.phase(), SessionPhase::AwaitLoginCompletion);
            s.login_completed(&id, LoginCompletion::Success, t).unwrap();
        }
        assert_eq!(s.login_report(), Some(LoginCompletion::Success));
        assert_eq!(s.phase(), SessionPhase::LoginReported);
        assert_eq!(s.failure(), None);
        s.finish(t).unwrap();
        assert_eq!(s.phase(), SessionPhase::Closed);
    }
}
#[test]
fn wrong_login_id_refuses_before_or_after_start_response() {
    for early in [true, false] {
        let (mut s, t, r, id) = starting();
        let wrong = s.login_id_for_test(b"SYNTHETIC_LOGIN_B").unwrap();
        if early {
            s.login_completed(&wrong, LoginCompletion::Success, t)
                .unwrap();
            assert_eq!(
                s.login_start_response(r.id(), &id, t),
                Err(SessionError::Correlation)
            );
        } else {
            s.login_start_response(r.id(), &id, t).unwrap();
            assert_eq!(
                s.login_completed(&wrong, LoginCompletion::Success, t),
                Err(SessionError::Correlation)
            );
        }
        assert_stopped(&mut s, t, SessionError::Correlation);
    }
}
#[test]
fn same_id_bytes_from_another_session_do_not_match() {
    let (mut s, t, _, _) = starting();
    let (other, _, _, id) = starting();
    drop(other);
    assert_eq!(
        s.login_completed(&id, LoginCompletion::Success, t),
        Err(SessionError::Correlation)
    );
    assert_stopped(&mut s, t, SessionError::Correlation);
}
#[test]
fn duplicate_and_conflicting_early_completions_are_not_last_writer_wins() {
    for result in [LoginCompletion::Success, LoginCompletion::Failure] {
        let (mut s, t, _, id) = starting();
        s.login_completed(&id, LoginCompletion::Success, t).unwrap();
        assert_eq!(
            s.login_completed(&id, result, t),
            Err(SessionError::Correlation)
        );
        assert_stopped(&mut s, t, SessionError::Correlation);
    }
}
#[test]
fn duplicate_start_response_and_post_completion_message_refuse() {
    let (mut s, t, r, id) = starting();
    s.login_start_response(r.id(), &id, t).unwrap();
    assert!(s.login_start_response(r.id(), &id, t).is_err());
    assert_eq!(s.login_report(), None);
    let (mut s, t, id) = waiting();
    s.login_completed(&id, LoginCompletion::Success, t).unwrap();
    assert_eq!(
        s.login_completed(&id, LoginCompletion::Success, t),
        Err(SessionError::Sequence)
    );
    assert_stopped(&mut s, t, SessionError::Sequence);
}
#[test]
fn unsent_start_response_and_early_completion_refuse() {
    for completion in [true, false] {
        let (mut s, t) = ready();
        let id = s.login_id_for_test(b"SYNTHETIC").unwrap();
        let r = s.start_login(t).unwrap();
        let result = if completion {
            s.login_completed(&id, LoginCompletion::Success, t)
        } else {
            s.login_start_response(r.id(), &id, t)
        };
        assert_eq!(result, Err(SessionError::UnsentResponse));
        assert_stopped(&mut s, t, SessionError::UnsentResponse);
    }
}
#[test]
fn start_response_requires_original_request_ticket_and_specialized_handler() {
    let (mut s, t, _, id) = starting();
    let (_, _, other, _) = starting();
    assert_eq!(
        s.login_start_response(other.id(), &id, t),
        Err(SessionError::Correlation)
    );
    let (mut s, t, r, _) = starting();
    assert_eq!(
        s.response(r.id(), ResponseOutcome::Success, t),
        Err(SessionError::Sequence)
    );
    assert_stopped(&mut s, t, SessionError::Sequence);
}
#[test]
fn login_requires_sealed_lane_complete_handshake_and_no_pending_work() {
    let t = Instant::now();
    let mut s = ProtocolSession::for_test(t).unwrap();
    handshake(&mut s, t);
    assert_eq!(
        s.start_login(t).unwrap_err(),
        SessionError::MethodUnavailable
    );
    for step in 0..6 {
        let (mut s, t) = fresh();
        if step > 0 {
            let r = s.reserve(Method::Initialize, t).unwrap();
            if step > 1 {
                s.sent(r.id(), t).unwrap();
            }
            if step > 2 {
                s.response(r.id(), ResponseOutcome::Success, t).unwrap();
                s.initialized_sent(t).unwrap();
            }
            if step == 3 {
                s.reserve(Method::AccountRead, t).unwrap();
            }
            if step == 4 {
                s.account_notification(t).unwrap();
            }
            if step == 5 {
                s.start_login(t).unwrap();
            }
        }
        assert_eq!(s.start_login(t).unwrap_err(), SessionError::Sequence);
    }
}
#[test]
fn account_requests_and_generic_login_methods_remain_unavailable_during_login() {
    for method in [
        Method::AccountRead,
        Method::RateLimitsRead,
        Method::Initialize,
        Method::LoginStart,
        Method::LoginCancel,
        Method::Initialized,
    ] {
        let (mut s, t, _) = waiting();
        assert!(s.reserve(method, t).is_err());
        assert_eq!(s.phase(), SessionPhase::Stopped);
    }
}
#[test]
fn failed_completion_and_start_error_never_report_success() {
    for early in [true, false] {
        let (mut s, t, r, id) = starting();
        let result = if early {
            s.login_completed(&id, LoginCompletion::Failure, t).unwrap();
            s.login_start_response(r.id(), &id, t)
        } else {
            s.login_start_response(r.id(), &id, t).unwrap();
            s.login_completed(&id, LoginCompletion::Failure, t)
        };
        assert_eq!(result, Err(SessionError::LoginFailed));
        assert_stopped(&mut s, t, SessionError::LoginFailed);
    }
    let (mut s, t, r, id) = starting();
    s.login_completed(&id, LoginCompletion::Success, t).unwrap();
    assert_eq!(
        s.login_start_error(r.id(), t),
        Err(SessionError::RemoteError)
    );
    assert_stopped(&mut s, t, SessionError::RemoteError);
}
#[test]
fn combined_account_and_early_completion_queue_is_bounded_at_thirty_two() {
    for extra_completion in [true, false] {
        let (mut s, t, _, id) = starting();
        for _ in 0..31 {
            s.account_notification(t).unwrap();
        }
        s.login_completed(&id, LoginCompletion::Success, t).unwrap();
        let result = if extra_completion {
            let id2 = s.login_id_for_test(b"SYNTHETIC_B").unwrap();
            s.login_completed(&id2, LoginCompletion::Success, t)
        } else {
            s.account_notification(t)
        };
        assert_eq!(result, Err(SessionError::NotificationLimit));
        assert_stopped(&mut s, t, SessionError::NotificationLimit);
    }
}
#[test]
fn early_queue_drain_and_shared_notification_credit_do_not_fabricate_a_match() {
    let (mut s, t, r, id) = starting();
    for _ in 0..31 {
        s.account_notification(t).unwrap();
    }
    s.login_completed(&id, LoginCompletion::Success, t).unwrap();
    s.take_notification(t).unwrap();
    s.account_notification(t).unwrap();
    s.login_start_response(r.id(), &id, t).unwrap();
    assert_eq!(s.login_report(), Some(LoginCompletion::Success));
    assert_eq!(s.notifications, 31);
    assert_eq!(s.early_login_notifications(), 0);
    for _ in 0..31 {
        s.take_notification(t).unwrap();
    }
    s.finish(t).unwrap();
    let (mut s, t, r, id) = starting();
    for i in 0..32 {
        let other = s
            .login_id_for_test(format!("SYNTHETIC_{i}").as_bytes())
            .unwrap();
        s.login_completed(&other, LoginCompletion::Success, t)
            .unwrap();
    }
    assert_eq!(
        s.login_start_response(r.id(), &id, t),
        Err(SessionError::Correlation)
    );
}
#[test]
fn start_response_deadline_is_not_the_ten_minute_interactive_budget() {
    let (mut s, t, r, id) = starting();
    assert_eq!(r.deadline(), t + REQUEST);
    assert_eq!(
        s.login_start_response(r.id(), &id, t + REQUEST),
        Err(SessionError::RequestTimeout)
    );
    assert_stopped(&mut s, t + REQUEST, SessionError::RequestTimeout);
    let (mut s, t) = ready();
    let r = s.start_login(t + Duration::from_secs(29)).unwrap();
    assert_eq!(r.deadline(), t + ProtocolSession::TOTAL_BUDGET);
}
#[test]
fn interactive_wait_exceeds_thirty_seconds_without_resetting_ten_minute_deadline() {
    let (mut s, t, id) = waiting();
    for sec in [31, 120, 599] {
        let now = t + Duration::from_secs(sec);
        s.account_notification(now).unwrap();
        s.take_notification(now).unwrap();
        assert_eq!(s.poll(now), Ok(t + ProtocolSession::LOGIN_BUDGET));
    }
    let now = t + ProtocolSession::LOGIN_BUDGET - Duration::from_nanos(1);
    s.login_completed(&id, LoginCompletion::Success, now)
        .unwrap();
    assert_eq!(s.login_report(), Some(LoginCompletion::Success));
}
#[test]
fn exact_login_deadline_wins_over_completion_then_allows_only_bounded_cancel() {
    let (mut s, t, id) = waiting();
    let expiry = t + ProtocolSession::LOGIN_BUDGET;
    assert_eq!(
        s.login_completed(&id, LoginCompletion::Success, expiry),
        Err(SessionError::LoginTimeout)
    );
    assert_eq!(s.login_report(), None);
    assert_eq!(s.cancellation_state(), CancellationState::Ready);
    let r = s.reserve_login_cancel(expiry).unwrap().unwrap();
    assert_eq!(s.cancellation_target(), Some(&id));
    s.sent(r.id(), expiry).unwrap();
    assert_eq!(
        s.login_cancel_response(r.id(), ResponseOutcome::Success, expiry),
        Err(SessionError::LoginTimeout)
    );
    assert_eq!(s.cancellation_state(), CancellationState::Acknowledged);
    assert_eq!(s.cancellation_error(), None);
}
#[test]
fn cancel_before_start_send_has_no_matching_login_to_cancel() {
    for reserved in [true, false] {
        let (mut s, t) = ready();
        if reserved {
            s.start_login(t).unwrap();
        }
        assert_eq!(s.cancel(t), Err(SessionError::Cancelled));
        assert_eq!(s.cancellation_state(), CancellationState::Unavailable);
        assert!(s.cancellation_target().is_none());
        assert!(s.reserve_login_cancel(t).is_err());
        assert_stopped(&mut s, t, SessionError::Cancelled);
    }
}
#[test]
fn cancel_before_start_response_waits_boundedly_for_the_matching_id() {
    let (mut s, t, start, id) = starting();
    assert_eq!(s.cancel(t), Err(SessionError::Cancelled));
    assert_eq!(s.cancellation_state(), CancellationState::WaitingForId);
    assert!(s.reserve_login_cancel(t).unwrap().is_none());
    s.login_completed(&id, LoginCompletion::Success, t).unwrap();
    s.login_start_response(start.id(), &id, t).unwrap();
    assert_eq!(s.login_report(), None);
    let cancel = s.reserve_login_cancel(t).unwrap().unwrap();
    s.sent(cancel.id(), t).unwrap();
    assert_eq!(
        s.login_cancel_response(cancel.id(), ResponseOutcome::Success, t),
        Err(SessionError::Cancelled)
    );
    assert_eq!(s.cancellation_state(), CancellationState::Acknowledged);
    assert_eq!(s.failure(), Some(SessionError::Cancelled));
}
#[test]
fn late_start_response_after_cancel_wait_cannot_resurrect_the_login() {
    let (mut s, t, start, id) = starting();
    s.cancel(t).unwrap_err();
    let expiry = t + ProtocolSession::CANCELLATION_BUDGET;
    assert_eq!(
        s.login_start_response(start.id(), &id, expiry),
        Err(SessionError::Cancelled)
    );
    assert_eq!(s.cancellation_state(), CancellationState::TimedOut);
    assert_eq!(
        s.cancellation_error(),
        Some(SessionError::CancellationTimeout)
    );
    assert_stopped(&mut s, expiry, SessionError::Cancelled);
}
#[test]
fn cancel_ack_requires_one_sent_request_with_original_id() {
    for kind in 0..4 {
        let (mut s, t, _) = waiting();
        s.cancel(t).unwrap_err();
        let r = s.reserve_login_cancel(t).unwrap().unwrap();
        let result = match kind {
            0 => s.login_cancel_response(r.id(), ResponseOutcome::Success, t),
            1 => {
                s.sent(r.id(), t).unwrap();
                let (_, _, other, _) = starting();
                s.login_cancel_response(other.id(), ResponseOutcome::Success, t)
            }
            2 => s.reserve_login_cancel(t).map(|_| ()),
            _ => {
                s.sent(r.id(), t).unwrap();
                s.login_cancel_response(r.id(), ResponseOutcome::Error, t)
            }
        };
        assert_eq!(result, Err(SessionError::Cancelled));
        assert_eq!(s.cancellation_state(), CancellationState::Failed);
        assert_eq!(
            s.cancellation_error(),
            Some(
                [
                    SessionError::UnsentResponse,
                    SessionError::Correlation,
                    SessionError::DuplicateSend,
                    SessionError::RemoteError
                ][kind]
            )
        );
    }
}
#[test]
fn cancellation_and_success_order_never_overrides_the_user_intent() {
    for success_first in [true, false] {
        let (mut s, t, id) = waiting();
        if success_first {
            s.login_completed(&id, LoginCompletion::Success, t).unwrap();
        }
        assert_eq!(s.cancel(t), Err(SessionError::Cancelled));
        if !success_first {
            s.login_completed(&id, LoginCompletion::Success, t).unwrap();
            let r = s.reserve_login_cancel(t).unwrap().unwrap();
            s.sent(r.id(), t).unwrap();
            s.login_cancel_response(r.id(), ResponseOutcome::Success, t)
                .unwrap_err();
        }
        assert_eq!(s.login_report(), None);
        assert_eq!(s.failure(), Some(SessionError::Cancelled));
    }
}
#[test]
fn cancellation_progress_and_repeat_intent_cannot_extend_shutdown_deadline() {
    let (mut s, t, id) = waiting();
    s.cancel(t).unwrap_err();
    let expiry = t + ProtocolSession::CANCELLATION_BUDGET;
    assert_eq!(s.poll_cancellation(t), Ok(expiry));
    assert_eq!(
        s.cancel(t + Duration::from_secs(1)),
        Err(SessionError::Cancelled)
    );
    let r = s
        .reserve_login_cancel(t + Duration::from_secs(2))
        .unwrap()
        .unwrap();
    assert_eq!(r.deadline(), expiry);
    s.sent(r.id(), t + Duration::from_secs(3)).unwrap();
    s.login_completed(&id, LoginCompletion::Success, t + Duration::from_secs(4))
        .unwrap();
    assert_eq!(
        s.login_cancel_response(r.id(), ResponseOutcome::Success, expiry),
        Err(SessionError::Cancelled)
    );
    assert_eq!(s.cancellation_state(), CancellationState::TimedOut);
    assert_eq!(
        s.cancellation_error(),
        Some(SessionError::CancellationTimeout)
    );
}
#[test]
fn malformed_transport_and_eof_during_cancel_preserve_primary_and_secondary() {
    for kind in 0..4 {
        let (mut s, t, _) = waiting();
        s.cancel(t).unwrap_err();
        let r = match kind {
            0 => s.invalid_message(t),
            1 => s.transport_failed(t),
            2 => s.unexpected_server_request(t),
            _ => s.finish(t),
        };
        assert_eq!(r, Err(SessionError::Cancelled));
        assert_eq!(s.cancellation_state(), CancellationState::Failed);
        assert_eq!(
            s.cancellation_error(),
            Some(
                [
                    SessionError::InvalidMessage,
                    SessionError::TransportFailure,
                    SessionError::UnexpectedServerRequest,
                    SessionError::IncompleteEof
                ][kind]
            )
        );
        assert_stopped(&mut s, t, SessionError::Cancelled);
    }
}
#[test]
fn backwards_clock_and_forward_jump_do_not_bypass_login_or_cancel_timeout() {
    let (mut s, t, _) = waiting();
    s.poll(t + Duration::from_secs(40)).unwrap();
    assert_eq!(s.poll(t), Err(SessionError::ClockRegression));
    let (mut s, t, _) = waiting();
    assert_eq!(
        s.poll(t + Duration::from_secs(1000)),
        Err(SessionError::LoginTimeout)
    );
    assert_eq!(
        s.poll_cancellation(t + Duration::from_secs(1006)),
        Err(SessionError::LoginTimeout)
    );
    assert_eq!(
        s.cancellation_error(),
        Some(SessionError::CancellationTimeout)
    );
    let (mut s, t, _) = waiting();
    s.cancel(t + Duration::from_secs(2)).unwrap_err();
    assert_eq!(s.poll_cancellation(t), Err(SessionError::Cancelled));
    assert_eq!(s.cancellation_error(), Some(SessionError::ClockRegression));
}
#[test]
fn login_ids_are_exact_bounded_redacted_and_never_caller_constructible() {
    let (s, _, _, id) = starting();
    assert!(s.login_id_for_test(b"").is_err());
    assert!(s.login_id_for_test(&[b'x'; 257]).is_err());
    assert!(s.login_id_for_test(&[b'x'; 256]).is_ok());
    assert_eq!(format!("{id:?}"), "LoginId([REDACTED])");
    let a = s.login_id_for_test(b"SYNTHETIC_A ").unwrap();
    let b = s.login_id_for_test(b"SYNTHETIC_A").unwrap();
    assert_ne!(a, b);
    let (mut s, t, r, _) = starting();
    let invalid = LoginId {
        owner: s.owner.clone(),
        bytes: Zeroizing::new(Vec::new()),
    };
    assert_eq!(
        s.login_start_response(r.id(), &invalid, t),
        Err(SessionError::LoginIdInvalid)
    );
    assert_stopped(&mut s, t, SessionError::LoginIdInvalid);
}
#[test]
fn success_and_cancel_remain_terminal_without_a_second_login_attempt() {
    let (mut s, t, id) = waiting();
    s.login_completed(&id, LoginCompletion::Success, t).unwrap();
    assert_eq!(s.start_login(t).unwrap_err(), SessionError::Sequence);
    assert_stopped(&mut s, t, SessionError::Sequence);
    let (mut s, t, id) = waiting();
    s.login_completed(&id, LoginCompletion::Success, t).unwrap();
    s.finish(t).unwrap();
    assert_eq!(s.start_login(t).unwrap_err(), SessionError::Closed);
    assert_eq!(s.cancel(t), Err(SessionError::Closed));
}
#[test]
fn lost_start_or_completion_eof_never_becomes_a_reported_login() {
    let (mut s, t, _, _) = starting();
    assert_eq!(s.finish(t), Err(SessionError::IncompleteEof));
    assert_stopped(&mut s, t, SessionError::IncompleteEof);
    let (mut s, t, _) = waiting();
    assert_eq!(s.finish(t), Err(SessionError::IncompleteEof));
    assert_stopped(&mut s, t, SessionError::IncompleteEof);
}

#[test]
fn all_six_start_completion_cancel_orders_preserve_cancel_intent() {
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let (mut s, t, start, id) = starting();
        for event in order {
            match event {
                0 => s.login_start_response(start.id(), &id, t).unwrap(),
                1 => s.login_completed(&id, LoginCompletion::Success, t).unwrap(),
                _ => assert_eq!(s.cancel(t), Err(SessionError::Cancelled)),
            }
        }
        if s.phase() == SessionPhase::LoginCancelling {
            let r = s.reserve_login_cancel(t).unwrap().unwrap();
            s.sent(r.id(), t).unwrap();
            assert_eq!(
                s.login_cancel_response(r.id(), ResponseOutcome::Success, t),
                Err(SessionError::Cancelled)
            );
        }
        assert_eq!(s.failure(), Some(SessionError::Cancelled));
        assert_eq!(s.login_report(), None);
        assert_eq!(s.phase(), SessionPhase::Stopped);
        assert_eq!(
            s.login_completed(&id, LoginCompletion::Success, t),
            Err(SessionError::Cancelled)
        );
    }
}
#[test]
fn cancellation_wait_is_capped_by_the_original_start_response_deadline() {
    let (mut s, t, start, id) = starting();
    let now = t + Duration::from_secs(14);
    s.cancel(now).unwrap_err();
    assert_eq!(s.poll_cancellation(now), Ok(start.deadline()));
    s.login_start_response(start.id(), &id, now).unwrap();
    let r = s.reserve_login_cancel(now).unwrap().unwrap();
    assert_eq!(r.deadline(), start.deadline());
    assert_eq!(
        s.sent(r.id(), start.deadline()),
        Err(SessionError::Cancelled)
    );
    assert_eq!(s.cancellation_state(), CancellationState::TimedOut);
}
#[test]
fn repeat_cancel_observes_clock_and_timeout_without_resetting_primary() {
    let (mut s, t, _) = waiting();
    s.cancel(t).unwrap_err();
    assert_eq!(
        s.cancel(t + ProtocolSession::CANCELLATION_BUDGET),
        Err(SessionError::Cancelled)
    );
    assert_eq!(s.cancellation_state(), CancellationState::TimedOut);
    assert_eq!(
        s.cancellation_error(),
        Some(SessionError::CancellationTimeout)
    );
}
#[test]
fn cancellation_cannot_bypass_the_lifetime_request_limit() {
    let (mut s, t) = ready();
    for _ in 0..MAX_ISSUED - 2 {
        let r = s.reserve(Method::AccountRead, t).unwrap();
        s.sent(r.id(), t).unwrap();
        s.response(r.id(), ResponseOutcome::Success, t).unwrap();
    }
    let id = s.login_id_for_test(b"SYNTHETIC").unwrap();
    let start = s.start_login(t).unwrap();
    s.sent(start.id(), t).unwrap();
    s.login_start_response(start.id(), &id, t).unwrap();
    s.cancel(t).unwrap_err();
    assert_eq!(
        s.reserve_login_cancel(t).unwrap_err(),
        SessionError::Cancelled
    );
    assert_eq!(s.cancellation_state(), CancellationState::Failed);
    assert_eq!(s.cancellation_error(), Some(SessionError::IssuedLimit));
}
