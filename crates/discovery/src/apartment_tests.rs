//! Known synthetic bytes only. Run each exact case in a fresh supervised process
//! to exercise final-MTA teardown without other tests retaining that apartment.
#![forbid(unsafe_code)]

use super::{hash_bytes, Apartment};
use std::sync::{Arc, Barrier};

const EXPECTED: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn registered_activation_retries_only_server_stopping_with_a_fixed_bound() {
    use windows::core::{Error, HRESULT};
    for (code, failures, expected_calls) in [
        (0x80080008u32, 2, 3),
        (0x80080008, 10, 4),
        (0x80070005, 10, 1),
        (0x80040154, 10, 1),
        (0x80010106, 10, 1),
    ] {
        let mut calls = 0;
        let mut pauses = 0;
        let result = crate::native::activate_registered(
            || {
                calls += 1;
                if calls <= failures {
                    Err(Error::from_hresult(HRESULT(code as i32)))
                } else {
                    Ok(42)
                }
            },
            |delay| {
                assert_eq!(delay, std::time::Duration::from_millis(50));
                pauses += 1;
            },
        );
        assert_eq!(calls, expected_calls);
        assert_eq!(pauses, expected_calls - 1);
        if failures < expected_calls {
            assert_eq!(result.unwrap(), 42);
        } else {
            assert_eq!(result.unwrap_err().code().0, code as i32);
        }
    }
}

#[test]
fn scoped_sha256_preserves_empty_and_chunked_vectors_without_caller_apartment() {
    // The owner retains its own apartment; the safe API never exposes WinRT
    // objects requiring a caller's unexpressed lifetime promise.
    let empty = crate::native::Sha256::new().expect("synthetic digest initialization");
    assert_eq!(
        empty.finish().expect("empty synthetic digest"),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    drop(empty);
    let chunked = crate::native::Sha256::new().expect("synthetic digest initialization");
    for bytes in [b"".as_slice(), b"a", b"bc"] {
        chunked.append(bytes).expect("synthetic chunk append");
    }
    assert_eq!(
        chunked.finish().expect("chunked synthetic digest"),
        EXPECTED
    );
}

#[test]
fn hash_survives_sequential_apartment_teardown_and_reinitialization() {
    for _ in 0..12 {
        // A fresh thread owns each apartment. Joining releases the entire wave
        // before the next initialization; only an owned String leaves hash_bytes.
        std::thread::spawn(|| {
            let _apartment = Apartment::enter().expect("synthetic apartment initialization");
            for _ in 0..2 {
                assert_eq!(hash_bytes(b"abc").expect("synthetic digest"), EXPECTED);
            }
        })
        .join()
        .expect("synthetic apartment worker completed");
    }
}

#[test]
fn hash_survives_concurrent_apartment_waves_and_final_teardown() {
    for _ in 0..4 {
        let barrier = Arc::new(Barrier::new(4));
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let apartment = Apartment::enter();
                    // Even a typed initialization refusal reaches the barrier;
                    // it cannot strand the other workers waiting for this one.
                    barrier.wait();
                    let _apartment = apartment.expect("synthetic apartment initialization");
                    for _ in 0..3 {
                        assert_eq!(hash_bytes(b"abc").expect("synthetic digest"), EXPECTED);
                    }
                })
            })
            .collect();
        // Join every worker before reporting a panic, so a failed assertion never
        // leaves apartment teardown running in the background of another case.
        let completed: Vec<_> = workers.into_iter().map(|worker| worker.join()).collect();
        assert!(
            completed.iter().all(Result::is_ok),
            "synthetic apartment wave completed"
        );
    }
}
