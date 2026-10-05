#![forbid(unsafe_code)]
//! Synthetic channel peers, never an official schema, OS pipe or helper process.
use super::*;
use std::{sync::mpsc::sync_channel, thread};

const SAMPLE: &[u8] = b"{\"synthetic\":true}\n";

struct Peer {
    input: ReadEnd,
    output: WriteEnd,
    error: WriteEnd,
}
fn harness(session: &ProtocolSession) -> (TransportDriver, Peer, io::CancelHandle) {
    let reader = Arc::new(());
    let (wake_tx, wake) = sync_channel(1);
    let (input, stdin) = io::pair(&reader, Stream::Input, &wake_tx);
    let (stdout, output) = io::pair(&reader, Stream::Output, &wake_tx);
    let (stderr, error) = io::pair(&reader, Stream::Error, &wake_tx);
    let cancel = io::cancellation(&wake_tx);
    let driver = TransportDriver {
        owner: session.owner.clone(),
        reader,
        stdout,
        stderr,
        stdin,
        wake,
        cancel: cancel.requested.clone(),
        decoder: JsonObjectDecoder::default(),
        chunk: None,
        cursor: 0,
        ready: None,
        lease: Weak::new(),
        write: None,
        stderr_total: 0,
        stdout_eof: false,
        stderr_eof: false,
        cancel_seen: false,
        cancel_notified: false,
        closed: false,
        first_error: None,
    };
    (
        driver,
        Peer {
            input,
            output,
            error,
        },
        cancel,
    )
}
fn bytes(input: &[u8]) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(input.to_vec())
}
fn take_frame(driver: &mut TransportDriver, session: &mut ProtocolSession) -> ReaderFrame {
    match driver.next(session).unwrap() {
        DriverEvent::Frame(frame) => frame,
        _ => panic!("expected synthetic frame"),
    }
}
fn written(driver: &mut TransportDriver, session: &mut ProtocolSession, method: Method) {
    assert!(matches!(driver.next(session).unwrap(), DriverEvent::Written(m) if m == method));
}
fn ready(session: &mut ProtocolSession) {
    let request = session.reserve(Method::Initialize, Instant::now()).unwrap();
    session.sent(request.id(), Instant::now()).unwrap();
    session
        .response(request.id(), ResponseOutcome::Success, Instant::now())
        .unwrap();
    session.initialized_sent(Instant::now()).unwrap();
}
fn initialize(session: &mut ProtocolSession, driver: &mut TransportDriver) -> IssuedRequest {
    let request = session.reserve(Method::Initialize, Instant::now()).unwrap();
    driver
        .queue_request(session, &request, bytes(SAMPLE))
        .unwrap();
    request
}
fn large_object(size: usize) -> Zeroizing<Vec<u8>> {
    let mut value = b"{\"synthetic\":\"".to_vec();
    value.resize(size - 2, b'x');
    value.extend_from_slice(b"\"}\n");
    Zeroizing::new(value)
}
// Every synthetic producer/consumer is bounded and joined. The driver itself has
// no thread, spawn, blocking data write or detached worker to clean up.
fn send_all(endpoint: &WriteEnd, input: &[u8]) {
    let deadline = Instant::now() + Duration::from_secs(10);
    for chunk in input.chunks(CHUNK_BYTES) {
        loop {
            assert!(Instant::now() < deadline, "synthetic producer deadline");
            match endpoint.try_write(chunk) {
                Ok(true) => break,
                Ok(false) => thread::yield_now(),
                Err(_) => panic!("synthetic endpoint closed"),
            }
        }
    }
}

#[test]
fn complete_handshake_receipts_and_eof_remain_protocol_only() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, mut peer, _cancel) = harness(&session);
    let request = initialize(&mut session, &mut driver);
    assert!(!session.pending[0].sent);
    written(&mut driver, &mut session, Method::Initialize);
    assert!(session.pending[0].sent);
    assert!(matches!(peer.input.try_read().unwrap(), ReadStep::Data(_)));
    assert!(peer.output.try_write(SAMPLE).unwrap());
    let frame = take_frame(&mut driver, &mut session);
    assert_eq!(
        frame.bytes(&driver, &session).unwrap(),
        &SAMPLE[..SAMPLE.len() - 1]
    );
    // Test-only normalized mapping: these bytes are NOT an official RPC response.
    session
        .response(request.id(), ResponseOutcome::Success, Instant::now())
        .unwrap();
    drop(frame);
    driver
        .queue_initialized(&mut session, bytes(SAMPLE))
        .unwrap();
    assert_eq!(session.phase(), SessionPhase::AwaitInitialized);
    written(&mut driver, &mut session, Method::Initialized);
    assert_eq!(session.phase(), SessionPhase::Ready);
    let account = session
        .reserve(Method::AccountRead, Instant::now())
        .unwrap();
    driver
        .queue_request(&mut session, &account, bytes(SAMPLE))
        .unwrap();
    written(&mut driver, &mut session, Method::AccountRead);
    assert!(peer.output.try_write(SAMPLE).unwrap());
    let frame = take_frame(&mut driver, &mut session);
    session
        .response(account.id(), ResponseOutcome::Success, Instant::now())
        .unwrap();
    drop(frame);
    driver.close_input(&mut session).unwrap();
    peer.output.close();
    peer.error.close();
    assert!(matches!(
        driver.next(&mut session).unwrap(),
        DriverEvent::Closed
    ));
    assert!(matches!(
        driver.next(&mut session).unwrap(),
        DriverEvent::Closed
    ));
    assert_eq!(session.phase(), SessionPhase::Closed);
}

#[test]
fn silent_endpoints_wake_at_the_existing_absolute_deadline() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, _peer, _cancel) = harness(&session);
    let deadline = Instant::now() + Duration::from_millis(40);
    session.initialize_deadline = deadline; // Shorten only this test's clock fixture.
    let error = driver.next(&mut session).unwrap_err();
    assert!(Instant::now() >= deadline);
    assert_eq!(error.primary, SessionError::InitializationTimeout);
    assert_eq!(driver.next(&mut session).unwrap_err(), error);
}

#[test]
fn a_separate_thread_cancels_silent_endpoints_without_waiting_for_data() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, _peer, cancel) = harness(&session);
    thread::scope(|scope| {
        let worker = scope.spawn(move || {
            thread::sleep(Duration::from_millis(20));
            cancel.request();
        });
        let error = driver.next(&mut session).unwrap_err();
        assert_eq!(error.primary, SessionError::Cancelled);
        worker.join().unwrap();
    });
}

#[test]
fn cancellation_is_not_lost_when_both_data_queues_and_wake_are_full() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, cancel) = harness(&session);
    initialize(&mut session, &mut driver);
    for _ in 0..io::QUEUE_CHUNKS {
        assert!(peer.output.try_write(SAMPLE).unwrap());
        assert!(peer.error.try_write(b"SYNTHETIC_STDERR_CANARY").unwrap());
    }
    assert!(!peer.output.try_write(SAMPLE).unwrap());
    cancel.request();
    assert_eq!(
        driver.next(&mut session).unwrap_err().primary,
        SessionError::Cancelled
    );
    assert!(matches!(peer.input.try_read().unwrap(), ReadStep::Eof));
}

#[test]
fn expiry_precedes_already_queued_data_and_cancel_intent() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, cancel) = harness(&session);
    assert!(peer.output.try_write(SAMPLE).unwrap());
    cancel.request();
    session.initialize_deadline = Instant::now();
    assert_eq!(
        driver.next(&mut session).unwrap_err().primary,
        SessionError::InitializationTimeout
    );
}

#[test]
fn stderr_is_independent_binary_discard_not_protocol_or_diagnostics() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    ready(&mut session);
    let (mut driver, mut peer, _cancel) = harness(&session);
    let canary = b"SYNTHETIC_TOKEN https://synthetic.invalid/login SYNTHETIC_JWT\xff";
    for _ in 0..io::QUEUE_CHUNKS {
        assert!(peer.error.try_write(canary).unwrap());
    }
    assert!(peer.output.try_write(SAMPLE).unwrap());
    let frame = take_frame(&mut driver, &mut session);
    assert_eq!(driver.stderr_total, canary.len());
    assert!(!format!("{driver:?} {frame:?}").contains("SYNTHETIC"));
    drop(frame);
    peer.output.close();
    peer.error.close();
    assert!(matches!(
        driver.next(&mut session).unwrap(),
        DriverEvent::Closed
    ));
    assert_eq!(driver.stderr_total, canary.len() * io::QUEUE_CHUNKS);
}

#[test]
fn stderr_exact_lifetime_cap_accepts_then_one_more_refuses_stickily() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    let chunk = [0xff; CHUNK_BYTES];
    for _ in 0..STDERR_TOTAL_LIMIT / CHUNK_BYTES {
        assert!(peer.error.try_write(&chunk).unwrap());
        assert!(driver.stderr_step(&mut session).unwrap());
    }
    assert_eq!(driver.stderr_total, STDERR_TOTAL_LIMIT);
    assert!(peer.error.try_write(b"x").unwrap());
    let error = driver.next(&mut session).unwrap_err();
    assert_eq!(error.fault, DriverFault::StderrLimit);
    assert_eq!(driver.next(&mut session).unwrap_err(), error);
    assert!(!format!("{error:?}").contains("SYNTHETIC"));
}

#[test]
fn continuous_stderr_cannot_extend_a_protocol_deadline() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    session.initialize_deadline = Instant::now() + Duration::from_millis(40);
    thread::scope(|scope| {
        let producer = scope.spawn(move || {
            let end = Instant::now() + Duration::from_secs(10);
            while Instant::now() < end {
                if peer.error.try_write(b"x").is_err() {
                    return;
                }
                thread::yield_now();
            }
            panic!("synthetic producer did not observe closure");
        });
        assert_eq!(
            driver.next(&mut session).unwrap_err().primary,
            SessionError::InitializationTimeout
        );
        producer.join().unwrap();
    });
}

#[test]
fn blocked_input_times_out_and_never_acknowledges_a_partial_frame() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    let request = session.reserve(Method::Initialize, Instant::now()).unwrap();
    driver
        .queue_request(&mut session, &request, large_object(MAX_FRAME_BYTES))
        .unwrap();
    for _ in 0..io::QUEUE_CHUNKS {
        assert_eq!(driver.write_step(&mut session).unwrap(), (true, None));
    }
    assert!(!session.pending[0].sent);
    assert_eq!(driver.write_step(&mut session).unwrap(), (false, None));
    session.initialize_deadline = Instant::now() + Duration::from_millis(40);
    assert_eq!(
        driver.next(&mut session).unwrap_err().primary,
        SessionError::InitializationTimeout
    );
    let mut delivered = 0;
    while let ReadStep::Data(chunk) = peer.input.try_read().unwrap() {
        delivered += chunk.bytes.len();
        assert!(!chunk.bytes.contains(&b'\n'));
    }
    assert_eq!(delivered, CHUNK_BYTES * io::QUEUE_CHUNKS);
}

#[test]
fn released_input_credit_wakes_writer_and_only_full_delivery_marks_sent() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    let request = session.reserve(Method::Initialize, Instant::now()).unwrap();
    let output = large_object(MAX_FRAME_BYTES);
    let expected = output.len();
    driver
        .queue_request(&mut session, &request, output)
        .unwrap();
    thread::scope(|scope| {
        let consumer = scope.spawn(move || {
            let end = Instant::now() + Duration::from_secs(10);
            let mut delivered = 0;
            loop {
                assert!(Instant::now() < end, "synthetic consumer deadline");
                match peer.input.try_read().unwrap() {
                    ReadStep::Data(chunk) => {
                        delivered += chunk.bytes.len();
                        if chunk.bytes.last() == Some(&b'\n') {
                            return delivered;
                        }
                    }
                    ReadStep::Pending => thread::yield_now(),
                    ReadStep::Eof => panic!("incomplete synthetic write"),
                }
            }
        });
        written(&mut driver, &mut session, Method::Initialize);
        assert!(session.pending[0].sent);
        assert_eq!(consumer.join().unwrap(), expected);
    });
}

#[test]
fn closed_input_is_a_typed_failure_not_a_sent_receipt() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, mut peer, _cancel) = harness(&session);
    initialize(&mut session, &mut driver);
    peer.input.close();
    let error = driver.next(&mut session).unwrap_err();
    assert_eq!(error.fault, DriverFault::InputClosed);
    assert_eq!(error.primary, SessionError::TransportFailure);
}

#[test]
fn outbound_shapes_reject_before_any_write_and_do_not_echo_canaries() {
    for invalid in [
        bytes(b""),
        bytes(b"{}"),
        bytes(b"[]\n"),
        bytes(b"{}\n{}\n"),
        bytes(b"{\"x\":1,\"x\":2}\n"),
        bytes(b"{\"SYNTHETIC_OUTPUT_CANARY\":\xff}\n"),
        large_object(MAX_FRAME_BYTES + 1),
    ] {
        let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
        let (mut driver, peer, _cancel) = harness(&session);
        let request = session.reserve(Method::Initialize, Instant::now()).unwrap();
        let error = driver
            .queue_request(&mut session, &request, invalid)
            .unwrap_err();
        assert_eq!(error.fault, DriverFault::InvalidOutput);
        assert!(matches!(peer.input.try_read().unwrap(), ReadStep::Eof));
        assert!(!format!("{driver:?} {error:?}").contains("SYNTHETIC_OUTPUT"));
    }
}

#[test]
fn duplicate_outbound_queue_and_early_initialized_cannot_bypass_order() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, _peer, _cancel) = harness(&session);
    let request = initialize(&mut session, &mut driver);
    assert_eq!(
        driver
            .queue_request(&mut session, &request, bytes(SAMPLE))
            .unwrap_err()
            .fault,
        DriverFault::OutputBusy
    );
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, _peer, _cancel) = harness(&session);
    assert_eq!(
        driver
            .queue_initialized(&mut session, bytes(SAMPLE))
            .unwrap_err()
            .fault,
        DriverFault::Binding
    );
}

#[test]
fn partial_utf8_and_multiple_frames_keep_exact_bytes_and_order() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    let raw = "{\"synthetic\":\"雪\"}\r\n{\"second\":true}\n".as_bytes();
    let split = raw.iter().position(|b| *b == 0xe9).unwrap() + 1;
    assert!(peer.output.try_write(&raw[..split]).unwrap());
    assert!(peer.output.try_write(&raw[split..]).unwrap());
    for expected in ["{\"synthetic\":\"雪\"}\r".as_bytes(), b"{\"second\":true}"] {
        let frame = take_frame(&mut driver, &mut session);
        assert_eq!(frame.bytes(&driver, &session).unwrap(), expected);
    }
}

#[test]
fn a_leased_frame_cannot_create_an_unbounded_consumer_queue() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    assert!(peer.output.try_write(b"{}\n{}\n").unwrap());
    let frame = take_frame(&mut driver, &mut session);
    assert_eq!(
        driver.next(&mut session).unwrap_err().fault,
        DriverFault::OutstandingFrame
    );
    assert_eq!(frame.bytes(&driver, &session), Err(DriverFault::Binding));
}

#[test]
fn invalid_stdout_discards_following_frames_and_remains_terminal() {
    for malformed in [
        b"{\"x\":1,\"x\":2}\n{}\n".as_slice(),
        b"[]\n{}\n",
        b"\xff\n{}\n",
        b"\n{}\n",
        b"{\"SYNTHETIC_SECRET_CANARY\":}\n{}\n",
    ] {
        let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
        let (mut driver, peer, _cancel) = harness(&session);
        assert!(peer.output.try_write(malformed).unwrap());
        let error = driver.next(&mut session).unwrap_err();
        assert!(matches!(error.fault, DriverFault::InvalidFrame(_)));
        assert_eq!(error.primary, SessionError::InvalidMessage);
        assert_eq!(driver.next(&mut session).unwrap_err(), error);
        assert!(!format!("{driver:?} {error:?}").contains("SYNTHETIC_SECRET"));
        assert!(peer.output.try_write(SAMPLE).is_err());
    }
}

#[test]
fn exact_inbound_frame_limit_crosses_many_queue_refills() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    thread::scope(|scope| {
        let producer = scope.spawn(move || send_all(&peer.output, &large_object(MAX_FRAME_BYTES)));
        let frame = take_frame(&mut driver, &mut session);
        assert_eq!(
            frame.bytes(&driver, &session).unwrap().len(),
            MAX_FRAME_BYTES
        );
        producer.join().unwrap();
    });
}

#[test]
fn inbound_one_byte_over_limit_is_not_truncated_into_success() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    thread::scope(|scope| {
        let producer =
            scope.spawn(move || send_all(&peer.output, &large_object(MAX_FRAME_BYTES + 1)));
        let error = driver.next(&mut session).unwrap_err();
        assert_eq!(
            error.fault,
            DriverFault::InvalidFrame(JsonFrameError::Transport(crate::FrameError::FrameTooLarge))
        );
        producer.join().unwrap();
    });
}

#[test]
fn truncated_stdout_eof_refuses_even_if_stderr_is_still_open() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, mut peer, _cancel) = harness(&session);
    assert!(peer.output.try_write(b"{\"synthetic\":").unwrap());
    peer.output.close();
    assert_eq!(
        driver.next(&mut session).unwrap_err().fault,
        DriverFault::InvalidFrame(JsonFrameError::Transport(crate::FrameError::Truncated))
    );
}

#[test]
fn one_stream_eof_never_fakes_completion_of_the_other_stream() {
    for output_first in [true, false] {
        let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
        ready(&mut session);
        let (mut driver, mut peer, _cancel) = harness(&session);
        if output_first {
            peer.output.close();
        } else {
            peer.error.close();
        }
        session.total_deadline = Instant::now() + Duration::from_millis(40);
        assert_eq!(
            driver.next(&mut session).unwrap_err().primary,
            SessionError::TotalTimeout
        );
    }
}

#[test]
fn both_streams_eof_with_unanswered_request_is_incomplete() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, mut peer, _cancel) = harness(&session);
    initialize(&mut session, &mut driver);
    written(&mut driver, &mut session, Method::Initialize);
    peer.output.close();
    peer.error.close();
    assert_eq!(
        driver.next(&mut session).unwrap_err().primary,
        SessionError::IncompleteEof
    );
}

#[test]
fn foreign_session_reader_and_stream_swaps_refuse_even_without_data() {
    for swap in 0..3 {
        let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
        let other = ProtocolSession::for_test(Instant::now()).unwrap();
        let (mut driver, _peer, _cancel) = harness(&session);
        let (mut foreign, foreign_peer, _foreign_cancel) = harness(&other);
        drop(foreign_peer);
        match swap {
            0 => std::mem::swap(&mut driver.stdout, &mut foreign.stdout),
            1 => std::mem::swap(&mut driver.stdout, &mut driver.stderr),
            _ => std::mem::swap(&mut driver.stdin, &mut foreign.stdin),
        }
        assert_eq!(
            driver.next(&mut session).unwrap_err().fault,
            DriverFault::Binding
        );
    }
    let session = ProtocolSession::for_test(Instant::now()).unwrap();
    let mut other = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, _peer, _cancel) = harness(&session);
    assert_eq!(
        driver.next(&mut other).unwrap_err().fault,
        DriverFault::Binding
    );
}

#[test]
fn identical_bytes_from_another_private_reader_do_not_match() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    let (foreign, _foreign_peer, _foreign_cancel) = harness(&session);
    assert!(peer.output.try_write(SAMPLE).unwrap());
    let frame = take_frame(&mut driver, &mut session);
    assert_eq!(frame.bytes(&foreign, &session), Err(DriverFault::Binding));
    assert!(frame.bytes(&driver, &session).is_ok());
}

#[test]
fn cancellation_drives_matching_cancel_write_and_retains_ack_as_secondary() {
    let mut session = ProtocolSession::for_login_test(Instant::now()).unwrap();
    ready(&mut session);
    let start = session.start_login(Instant::now()).unwrap();
    session.sent(start.id(), Instant::now()).unwrap();
    let login = session.login_id_for_test(b"SYNTHETIC_LOGIN_ID").unwrap();
    session
        .login_start_response(start.id(), &login, Instant::now())
        .unwrap();
    let (mut driver, peer, cancel) = harness(&session);
    cancel.request();
    assert!(matches!(
        driver.next(&mut session).unwrap(),
        DriverEvent::CancellationRequired
    ));
    let request = session
        .reserve_login_cancel(Instant::now())
        .unwrap()
        .unwrap();
    driver
        .queue_request(&mut session, &request, bytes(SAMPLE))
        .unwrap();
    written(&mut driver, &mut session, Method::LoginCancel);
    assert!(peer.output.try_write(SAMPLE).unwrap());
    let frame = take_frame(&mut driver, &mut session);
    assert_eq!(
        session.login_cancel_response(request.id(), ResponseOutcome::Success, Instant::now()),
        Err(SessionError::Cancelled)
    );
    drop(frame);
    let error = driver.next(&mut session).unwrap_err();
    assert_eq!(error.primary, SessionError::Cancelled);
    assert_eq!(error.cancellation, CancellationState::Acknowledged);
    assert_eq!(error.cancellation_error, None);
    assert_eq!(session.login_report(), None);
}

#[test]
fn cancellation_wait_has_its_own_real_wakeup_and_never_resets_primary() {
    let mut session = ProtocolSession::for_login_test(Instant::now()).unwrap();
    ready(&mut session);
    let start = session.start_login(Instant::now()).unwrap();
    session.sent(start.id(), Instant::now()).unwrap();
    let (mut driver, _peer, cancel) = harness(&session);
    cancel.request();
    assert!(matches!(
        driver.next(&mut session).unwrap(),
        DriverEvent::CancellationRequired
    ));
    assert_eq!(
        session.cancellation_state(),
        CancellationState::WaitingForId
    );
    session.login.as_mut().unwrap().cancel_deadline =
        Some(Instant::now() + Duration::from_millis(40));
    cancel.request();
    let error = driver.next(&mut session).unwrap_err();
    assert_eq!(error.primary, SessionError::Cancelled);
    assert_eq!(error.cancellation, CancellationState::TimedOut);
    assert_eq!(
        error.cancellation_error,
        Some(SessionError::CancellationTimeout)
    );
}

#[test]
fn cancelled_login_with_hostile_frame_keeps_primary_and_transport_failure_separate() {
    let mut session = ProtocolSession::for_login_test(Instant::now()).unwrap();
    ready(&mut session);
    let start = session.start_login(Instant::now()).unwrap();
    session.sent(start.id(), Instant::now()).unwrap();
    let (mut driver, peer, cancel) = harness(&session);
    cancel.request();
    assert!(matches!(
        driver.next(&mut session).unwrap(),
        DriverEvent::CancellationRequired
    ));
    assert!(peer.output.try_write(b"[]\n").unwrap());
    let error = driver.next(&mut session).unwrap_err();
    assert_eq!(error.primary, SessionError::Cancelled);
    assert_eq!(error.cancellation, CancellationState::Failed);
    assert_eq!(error.cancellation_error, Some(SessionError::InvalidMessage));
}

#[test]
fn endpoint_chunk_limit_and_credit_are_exact_and_errors_are_redacted() {
    let session = ProtocolSession::for_test(Instant::now()).unwrap();
    let (mut driver, peer, _cancel) = harness(&session);
    assert_eq!(peer.output.try_write(&[]), Err(io::IoError::ChunkLimit));
    assert_eq!(
        peer.output.try_write(&vec![b'x'; CHUNK_BYTES + 1]),
        Err(io::IoError::ChunkLimit)
    );
    for _ in 0..io::QUEUE_CHUNKS {
        assert!(peer.output.try_write(&[b'x'; CHUNK_BYTES]).unwrap());
    }
    assert!(!peer.output.try_write(b"SYNTHETIC_CANARY").unwrap());
    assert!(matches!(
        driver.stdout.try_read().unwrap(),
        ReadStep::Data(_)
    ));
    assert!(peer.output.try_write(b"SYNTHETIC_CANARY").unwrap());
    driver.stdout.close();
    assert_eq!(
        peer.output.try_write(b"SYNTHETIC_CANARY"),
        Err(io::IoError::Closed)
    );
}

#[test]
fn a_closed_driver_cannot_clear_a_later_binding_failure() {
    let mut session = ProtocolSession::for_test(Instant::now()).unwrap();
    ready(&mut session);
    let (mut driver, mut peer, _cancel) = harness(&session);
    peer.output.close();
    peer.error.close();
    assert!(matches!(
        driver.next(&mut session).unwrap(),
        DriverEvent::Closed
    ));
    let mut foreign = ProtocolSession::for_test(Instant::now()).unwrap();
    let error = driver.next(&mut foreign).unwrap_err();
    assert_eq!(error.fault, DriverFault::Binding);
    assert_eq!(driver.next(&mut session).unwrap_err(), error);
}
