//! Synthetic syntax tests, not an official-runtime schema or native qualification.
use codex_accounts_runtime::{
    FrameError, JsonFrameError as Error, JsonObjectDecoder, MAX_FRAME_BYTES, MAX_QUEUED_FRAMES,
};

fn framed(payload: &[u8]) -> Vec<u8> {
    [payload, b"\n"].concat()
}
fn rejected(payload: &[u8], error: Error) {
    let mut decoder = JsonObjectDecoder::default();
    decoder.feed(&framed(payload)).unwrap();
    assert_eq!(decoder.next_object().unwrap_err(), error);
    assert_eq!(
        decoder.next_object().unwrap_err(),
        Error::Transport(FrameError::Poisoned)
    );
    assert_eq!(
        decoder.feed(b"{}\n"),
        Err(Error::Transport(FrameError::Poisoned))
    );
    assert_eq!(
        decoder.finish(),
        Err(Error::Transport(FrameError::Poisoned))
    );
}
fn large_object(size: usize) -> Vec<u8> {
    let mut bytes = b"{\"SYNTHETIC\":\"".to_vec();
    bytes.resize(size - 2, b'x');
    bytes.extend_from_slice(b"\"}");
    bytes
}

#[test]
fn every_byte_split_preserves_unicode_unknown_fields_and_cr() {
    let payload =
        " {\"SYNTHETIC\":\"\u{1f9ea}\u{6e2c}\",\"unknown\":[true,null,{\"v\":1}]}\r".as_bytes();
    let wire = framed(payload);
    for split in 0..=wire.len() {
        let mut d = JsonObjectDecoder::default();
        d.feed(&wire[..split]).unwrap();
        d.feed(&wire[split..]).unwrap();
        d.finish().unwrap();
        assert_eq!(d.next_object().unwrap().unwrap().as_bytes(), payload);
        assert!(d.next_object().unwrap().is_none());
    }
}

#[test]
fn malformed_scalar_batch_trailing_and_non_json_whitespace_refuse() {
    for input in [
        &b" "[..],
        b"null",
        b"true",
        b"42",
        b"\"SYNTHETIC\"",
        b"[]",
        b"[{}]",
        b"{",
        b"{broken}",
        b"{\"x\":}",
        b"{\"x\":1,}",
        b"{}{}",
        b"{}x",
        b"/*SYNTHETIC*/{}",
        b"{\"x\":NaN}",
        b"{\"x\":01}",
        b"{\"x\":1e400}",
        br#"{"x":"\uD800"}"#,
        br#"{"x":"\q"}"#,
        b"\xef\xbb\xbf{}",
        b"\x0b{}",
        b"{\"x\":\"SYNTHETIC\x00\"}",
    ] {
        rejected(input, Error::InvalidObject);
    }
}

#[test]
fn decoded_duplicate_keys_are_rejected_in_every_nested_object() {
    for input in [
        &br#"{"a":1,"a":2}"#[..],
        br#"{"a":1,"\u0061":2}"#,
        br#"{"nested":{"x":1,"x":2}}"#,
        br#"{"nested":[{"x":1,"x":2}]}"#,
        br#"{"\uD83E\uDDEA":1,"\ud83e\uddea":2}"#,
    ] {
        rejected(input, Error::DuplicateKey);
    }
    let mut d = JsonObjectDecoder::default();
    d.feed(b"{\"a\":{\"x\":1},\"b\":{\"x\":2}}\n").unwrap();
    assert!(d.next_object().unwrap().is_some());
}

#[test]
fn exactly_sixty_four_containers_are_allowed_not_sixty_five() {
    for count in [63, 64] {
        let payload = format!("{{\"x\":{}0{}}}", "[".repeat(count), "]".repeat(count));
        let mut d = JsonObjectDecoder::default();
        d.feed(&framed(payload.as_bytes())).unwrap();
        if count == 63 {
            assert!(d.next_object().unwrap().is_some());
        } else {
            assert_eq!(d.next_object().unwrap_err(), Error::DepthLimit);
        }
    }
}

#[test]
fn exact_frame_limit_roundtrips_and_one_extra_byte_poisoned() {
    let exact = large_object(MAX_FRAME_BYTES);
    let mut d = JsonObjectDecoder::default();
    d.feed(&framed(&exact)).unwrap();
    assert_eq!(d.next_object().unwrap().unwrap().as_bytes(), exact);
    assert_eq!(
        d.feed(&framed(&large_object(MAX_FRAME_BYTES + 1))),
        Err(Error::Transport(FrameError::FrameTooLarge))
    );
    assert!(d.next_object().is_err());
}

#[test]
fn syntax_failure_discards_queued_following_frames_and_partial_input() {
    let mut d = JsonObjectDecoder::default();
    d.feed(b"{}\n{\"x\":1,\"x\":2}\n{\"SYNTHETIC\":true}\n{\"partial\"")
        .unwrap();
    assert!(d.next_object().unwrap().is_some());
    assert_eq!(d.next_object().unwrap_err(), Error::DuplicateKey);
    assert!(d.next_object().is_err());
    assert!(d.feed(b":true}\n").is_err());
    assert!(d.finish().is_err());
}

#[test]
fn invalid_utf8_and_blank_lines_remain_transport_errors() {
    for (wire, error) in [
        (&b"{}\n\xff\n"[..], FrameError::InvalidUtf8),
        (&b"{}\n\n"[..], FrameError::EmptyFrame),
    ] {
        let mut d = JsonObjectDecoder::default();
        assert_eq!(d.feed(wire), Err(Error::Transport(error)));
        assert!(d.next_object().is_err());
    }
}

#[test]
fn eof_is_terminal_but_does_not_validate_queued_json() {
    let mut d = JsonObjectDecoder::default();
    d.feed(b"{broken}\n").unwrap();
    d.finish().unwrap();
    d.finish().unwrap();
    assert_eq!(d.next_object().unwrap_err(), Error::InvalidObject);
    let mut d = JsonObjectDecoder::default();
    d.feed(b"{}\n").unwrap();
    d.finish().unwrap();
    assert!(d.next_object().unwrap().is_some());
    assert!(d.next_object().unwrap().is_none());
    assert_eq!(d.feed(b""), Err(Error::Transport(FrameError::Closed)));
    assert!(d.next_object().is_err());
    let mut d = JsonObjectDecoder::default();
    d.feed(b"{}\n{").unwrap();
    assert_eq!(d.finish(), Err(Error::Transport(FrameError::Truncated)));
    assert!(d.next_object().is_err());
}

#[test]
fn pending_and_queue_bounds_cannot_be_bypassed_by_the_wrapper() {
    let mut d = JsonObjectDecoder::default();
    for _ in 0..MAX_QUEUED_FRAMES {
        d.feed(b"{}\n").unwrap();
    }
    d.next_object().unwrap().unwrap();
    d.feed(b"{}\n").unwrap();
    assert_eq!(
        d.feed(b"{}\n"),
        Err(Error::Transport(FrameError::QueueLimit))
    );
    let mut d = JsonObjectDecoder::default();
    let wire = framed(&large_object(MAX_FRAME_BYTES));
    for _ in 0..4 {
        d.feed(&wire).unwrap();
    }
    d.next_object().unwrap().unwrap();
    d.feed(&wire).unwrap();
    assert_eq!(
        d.feed(b"x"),
        Err(Error::Transport(FrameError::PendingLimit))
    );
    assert!(d.next_object().is_err());
}

#[test]
fn syntax_success_never_classifies_methods_or_response_identity() {
    for payload in [
        &br#"{"SYNTHETIC":true,"method":"turn/start"}"#[..],
        br#"{"SYNTHETIC":true,"id":"not-an-issued-id","result":{"qualified":true}}"#,
    ] {
        let mut d = JsonObjectDecoder::default();
        d.feed(&framed(payload)).unwrap();
        // Valid JSON is STILL hostile protocol input. No dispatcher is exposed.
        assert_eq!(d.next_object().unwrap().unwrap().as_bytes(), payload);
    }
}

#[test]
fn diagnostics_never_include_synthetic_payloads() {
    let mut d = JsonObjectDecoder::default();
    d.feed(b"{\"SYNTHETIC_SECRET\":\"SYNTHETIC_PRIVATE\"}\n")
        .unwrap();
    assert_eq!(format!("{d:?}"), "JsonObjectDecoder([REDACTED])");
    let frame = d.next_object().unwrap().unwrap();
    assert_eq!(format!("{frame:?}"), "JsonObjectFrame([REDACTED])");
    d.feed(b"{\"SYNTHETIC_SECRET\":1,\"SYNTHETIC_SECRET\":2}\n")
        .unwrap();
    assert_eq!(
        format!("{:?}", d.next_object().unwrap_err()),
        "DuplicateKey"
    );
}
