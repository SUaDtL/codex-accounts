//! Synthetic byte-stream tests only. No executable, adapter, or credential IO.
use codex_accounts_runtime::{
    ClientMessageKind, FrameDecoder, FrameError, Method, MAX_FRAME_BYTES,
    MAX_PENDING_BYTES, MAX_QUEUED_FRAMES,
};

fn assert_poisoned(decoder: &mut FrameDecoder) {
    assert!(decoder.next_frame().is_none());
    assert_eq!(decoder.feed(b"{}\n"), Err(FrameError::Poisoned));
    assert_eq!(decoder.feed(b""), Err(FrameError::Poisoned));
    assert_eq!(decoder.finish(), Err(FrameError::Poisoned));
}

#[test]
fn every_two_chunk_split_preserves_utf8_and_order() {
    let stream = "{\"synthetic\":\"雪🦀é\"}\n{\"synthetic\":2}\r\n".as_bytes();
    for split in 0..=stream.len() {
        let mut d = FrameDecoder::default();
        d.feed(&stream[..split]).unwrap();
        d.feed(&stream[split..]).unwrap();
        d.finish().unwrap();
        assert_eq!(d.next_frame().unwrap().as_bytes(), "{\"synthetic\":\"雪🦀é\"}".as_bytes());
        // A CR is preserved as untrusted payload, not normalized or interpreted.
        assert_eq!(d.next_frame().unwrap().as_bytes(), b"{\"synthetic\":2}\r");
        assert!(d.next_frame().is_none());
    }
}

#[test]
fn byte_at_a_time_and_empty_chunks_do_not_create_frames() {
    let mut d = FrameDecoder::default();
    for b in "雪🦀\n".as_bytes() {
        d.feed(b"").unwrap();
        d.feed(&[*b]).unwrap();
    }
    assert_eq!(d.next_frame().unwrap().as_bytes(), "雪🦀".as_bytes());
    d.finish().unwrap();
}

#[test]
fn eof_is_terminal_and_idempotent_but_queued_frames_can_drain() {
    let mut d = FrameDecoder::default();
    d.feed(b"first\nsecond\n").unwrap();
    d.finish().unwrap();
    d.finish().unwrap();
    assert_eq!(d.next_frame().unwrap().as_bytes(), b"first");
    assert_eq!(d.next_frame().unwrap().as_bytes(), b"second");
    assert!(d.next_frame().is_none());
    assert_eq!(d.feed(b"third\n"), Err(FrameError::Closed));
    assert_poisoned(&mut d);
}

#[test]
fn post_eof_injection_discards_undelivered_frames() {
    for tail in [b"".as_slice(), b"\n", b"forged\n"] {
        let mut d = FrameDecoder::default();
        d.feed(b"queued\n").unwrap();
        d.finish().unwrap();
        assert_eq!(d.feed(tail), Err(FrameError::Closed));
        assert_poisoned(&mut d);
    }
}

#[test]
fn draining_a_frame_restores_exact_aggregate_capacity() {
    let mut d = FrameDecoder::default();
    let body = vec![b'x'; MAX_FRAME_BYTES];
    for _ in 0..MAX_PENDING_BYTES / MAX_FRAME_BYTES {
        d.feed(&body).unwrap();
        d.feed(b"\n").unwrap();
    }
    assert_eq!(d.next_frame().unwrap().as_bytes().len(), MAX_FRAME_BYTES);
    d.feed(&body).unwrap();
    d.feed(b"\n").unwrap();
    assert_eq!(d.feed(b"x"), Err(FrameError::PendingLimit));
    assert_poisoned(&mut d);
}

#[test]
fn pending_partial_and_queued_frames_share_the_same_limit() {
    let mut d = FrameDecoder::default();
    let body = vec![b'x'; MAX_FRAME_BYTES];
    for _ in 0..3 {
        d.feed(&body).unwrap();
        d.feed(b"\n").unwrap();
    }
    d.feed(&body[..MAX_FRAME_BYTES - 1]).unwrap();
    d.feed(b"x").unwrap();
    d.feed(b"\n").unwrap();
    assert_eq!(d.feed(b"x"), Err(FrameError::PendingLimit));
    assert_poisoned(&mut d);
}

#[test]
fn queue_credit_is_reusable_but_not_unbounded() {
    let mut d = FrameDecoder::default();
    for _ in 0..MAX_QUEUED_FRAMES {
        d.feed(b"x\n").unwrap();
    }
    for _ in 0..100 {
        assert_eq!(d.next_frame().unwrap().as_bytes(), b"x");
        d.feed(b"x\n").unwrap();
    }
    assert_eq!(d.feed(b"x\n"), Err(FrameError::QueueLimit));
    assert_poisoned(&mut d);
}

#[test]
fn every_error_invalidates_an_earlier_queued_frame() {
    for (tail, expected) in [
        (vec![b'\n'], FrameError::EmptyFrame),
        (vec![0xff, b'\n'], FrameError::InvalidUtf8),
        (vec![b'x'; MAX_FRAME_BYTES + 1], FrameError::FrameTooLarge),
    ] {
        let mut d = FrameDecoder::default();
        d.feed(b"must-not-escape\n").unwrap();
        assert_eq!(d.feed(&tail), Err(expected));
        assert_poisoned(&mut d);
    }
    let mut d = FrameDecoder::default();
    d.feed(b"must-not-escape\ntruncated").unwrap();
    assert_eq!(d.finish(), Err(FrameError::Truncated));
    assert_poisoned(&mut d);
}

#[test]
fn invalid_utf8_variants_are_rejected_at_each_split() {
    for body in [
        vec![0xc0, 0xaf], vec![0xed, 0xa0, 0x80], vec![0xf4, 0x90, 0x80, 0x80],
        vec![0xe2, 0x82], vec![0x80],
    ] {
        for split in 0..=body.len() {
            let mut d = FrameDecoder::default();
            d.feed(&body[..split]).unwrap();
            d.feed(&body[split..]).unwrap();
            assert_eq!(d.feed(b"\n"), Err(FrameError::InvalidUtf8));
            assert_poisoned(&mut d);
        }
    }
}

#[test]
fn utf8_limits_are_byte_limits_not_character_limits() {
    let mut body = vec![b'x'; MAX_FRAME_BYTES - 4];
    body.extend_from_slice("🦀".as_bytes());
    let mut d = FrameDecoder::default();
    d.feed(&body).unwrap();
    d.feed(b"\n").unwrap();
    assert_eq!(d.next_frame().unwrap().as_bytes().len(), MAX_FRAME_BYTES);
    body.push(b'x');
    assert_eq!(d.feed(&body), Err(FrameError::FrameTooLarge));
    assert_poisoned(&mut d);
}

#[test]
fn framing_never_promotes_json_shaped_hostility_to_validated_protocol() {
    // Deliberately NOT an official protocol schema. Even malformed JSON, duplicate
    // keys and privileged-shaped messages remain UntrustedFrame, never responses.
    for body in [
        "{\"id\":1,\"id\":2}", "{\"method\":\"exec\"}", "[1,2]", "null", "{broken}",
        "{\"synthetic\":\"\\n\"}", "\u{feff}{}", "\0", "\r", " ",
    ] {
        let mut d = FrameDecoder::default();
        d.feed(body.as_bytes()).unwrap();
        d.feed(b"\n").unwrap();
        let frame = d.next_frame().unwrap();
        assert_eq!(frame.as_bytes(), body.as_bytes());
        assert_eq!(format!("{frame:?}"), "UntrustedFrame([REDACTED])");
    }
}

#[test]
fn all_allowlist_entries_have_one_outbound_message_kind() {
    for method in [Method::Initialize, Method::Initialized, Method::AccountRead,
        Method::LoginStart, Method::LoginCancel, Method::RateLimitsRead] {
        assert_eq!(Method::from_allowlist(method.wire_name()), Some(method));
        let kind = if method == Method::Initialized { ClientMessageKind::Notification }
            else { ClientMessageKind::Request };
        assert_eq!(Method::classify_client(method.wire_name(), kind), Some(method));
        let other = if kind == ClientMessageKind::Request { ClientMessageKind::Notification }
            else { ClientMessageKind::Request };
        assert_eq!(Method::classify_client(method.wire_name(), other), None);
    }
}

#[test]
fn lookalikes_and_unqualified_extensions_are_not_allowlisted() {
    for method in [Method::Initialize, Method::Initialized, Method::AccountRead,
        Method::LoginStart, Method::LoginCancel, Method::RateLimitsRead] {
        for candidate in [format!(" {}", method.wire_name()), format!("{} ", method.wire_name()),
            format!("{}\0", method.wire_name()), format!("{}\n", method.wire_name()),
            format!("{}/extra", method.wire_name()), method.wire_name().to_uppercase(),
            format!("{}\u{200b}", method.wire_name())] {
            assert_eq!(Method::from_allowlist(&candidate), None);
        }
    }
    for name in ["account%2Fread", "аccount/read", "account//read", "account\\read",
        "config/read", "configRequirements/read", "account/token/refresh", "account/logout",
        "thread/start", "turn/start", "command/exec", "fs/read", "mcp/call", "", "read"] {
        assert_eq!(Method::from_allowlist(name), None);
        assert_eq!(Method::classify_client(name, ClientMessageKind::Request), None);
        assert_eq!(Method::classify_client(name, ClientMessageKind::Notification), None);
    }
    assert_eq!(Method::from_allowlist(&"x".repeat(MAX_FRAME_BYTES)), None);
}

#[test]
fn chunk_schedules_preserve_drain_semantics() {
    // Deterministic stress corpus: no RNG dependency or timing-sensitive assertions.
    let stream = "一\ntwo\n🦀\nfour\n".as_bytes();
    for seed in 1..=128usize {
        let mut d = FrameDecoder::default();
        let mut pos = 0;
        let mut results = Vec::new();
        while pos < stream.len() {
            let end = (pos + 1 + ((pos * 17 + seed) % 7)).min(stream.len());
            d.feed(&stream[pos..end]).unwrap();
            while let Some(frame) = d.next_frame() {
                results.push(frame.as_bytes().to_vec());
            }
            pos = end;
        }
        d.finish().unwrap();
        assert_eq!(results, vec!["一".as_bytes(), b"two", "🦀".as_bytes(), b"four"]);
    }
}
