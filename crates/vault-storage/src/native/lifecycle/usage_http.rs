//! A read-only request to the same fixed usage endpoint used by official Codex.
//! No OAuth exchange, redirects, cookies, file writes or credential maintenance.
use std::{ffi::c_void, ptr::null, ptr::null_mut, time::Duration, time::Instant};
use windows_sys::Win32::Networking::WinHttp::*;
use zeroize::Zeroizing;

const UNAVAILABLE: &str = "E_USAGE_UNAVAILABLE";
const MAX_BODY: usize = 1024 * 1024;
const BUDGET: Duration = Duration::from_secs(20);

struct Handle(*mut c_void);
impl Handle {
    fn new(value: *mut c_void) -> Result<Self, &'static str> {
        if value.is_null() {
            Err(UNAVAILABLE)
        } else {
            Ok(Self(value))
        }
    }

    fn option(&self, option: u32, value: u32) -> Result<(), &'static str> {
        // SAFETY: live owned handle and a DWORD matching each selected option.
        success(unsafe {
            WinHttpSetOption(
                self.0,
                option,
                (&value as *const u32).cast(),
                std::mem::size_of::<u32>() as u32,
            )
        })
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: each non-null handle is owned once; child handles drop first.
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn success(value: i32) -> Result<(), &'static str> {
    if value == 0 {
        Err(UNAVAILABLE)
    } else {
        Ok(())
    }
}

fn headers(access_token: &str, account_id: &str) -> Result<Zeroizing<Vec<u16>>, &'static str> {
    // Do not permit a captured field to inject headers or an embedded terminator.
    if access_token.is_empty()
        || access_token.len() > 65536
        || !access_token
            .bytes()
            .all(|byte| (b'!'..=b'~').contains(&byte))
        || account_id.is_empty()
        || account_id.len() > 256
        || !account_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(UNAVAILABLE);
    }
    let text = Zeroizing::new(format!(
        "Authorization: Bearer {access_token}\r\nChatGPT-Account-Id: {account_id}\r\nAccept: application/json\r\n"
    ));
    Ok(Zeroizing::new(text.encode_utf16().chain([0]).collect()))
}

fn within_budget(started: Instant) -> Result<(), &'static str> {
    if started.elapsed() >= BUDGET {
        Err(UNAVAILABLE)
    } else {
        Ok(())
    }
}

pub(super) fn fetch(
    access_token: &str,
    account_id: &str,
) -> Result<Zeroizing<Vec<u8>>, &'static str> {
    let headers = headers(access_token, account_id)?;
    let started = Instant::now();
    // Default proxy uses Windows' configured static proxy. It performs no PAC
    // discovery; automatic server/proxy authentication is disabled below.
    // SAFETY: fixed terminated strings, synchronous session and no callbacks.
    let session = Handle::new(unsafe {
        WinHttpOpen(
            windows_sys::w!("CodexAccounts/0.1"),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            null(),
            null(),
            0,
        )
    })?;
    // All phases have finite waits; the elapsed budget additionally bounds a
    // peer that continuously supplies small response chunks. One blocking phase
    // may finish after the elapsed budget, by at most its native timeout.
    success(unsafe { WinHttpSetTimeouts(session.0, 3000, 3000, 3000, 2000) })?;
    session.option(WINHTTP_OPTION_CONNECT_RETRIES, 1)?;
    // Use OS TLS policy and normal chain/hostname validation, without ignore flags.
    let connection =
        Handle::new(unsafe { WinHttpConnect(session.0, windows_sys::w!("chatgpt.com"), 443, 0) })?;
    let request = Handle::new(unsafe {
        WinHttpOpenRequest(
            connection.0,
            windows_sys::w!("GET"),
            windows_sys::w!("/backend-api/wham/usage"),
            null(),
            null(),
            null(),
            WINHTTP_FLAG_SECURE,
        )
    })?;
    request.option(
        WINHTTP_OPTION_DISABLE_FEATURE,
        WINHTTP_DISABLE_AUTHENTICATION | WINHTTP_DISABLE_COOKIES | WINHTTP_DISABLE_REDIRECTS,
    )?;
    request.option(
        WINHTTP_OPTION_REDIRECT_POLICY,
        WINHTTP_OPTION_REDIRECT_POLICY_NEVER,
    )?;
    request.option(WINHTTP_OPTION_MAX_RESPONSE_HEADER_SIZE, 64 * 1024)?;
    request.option(WINHTTP_OPTION_RECEIVE_RESPONSE_TIMEOUT, 5000)?;
    within_budget(started)?;
    // SAFETY: bounded terminated headers remain alive for the synchronous call;
    // an empty GET body, no context or callback. No automatic retry is made here.
    success(unsafe {
        WinHttpSendRequest(
            request.0,
            headers.as_ptr(),
            (headers.len() - 1) as u32,
            null(),
            0,
            0,
            0,
        )
    })?;
    within_budget(started)?;
    success(unsafe { WinHttpReceiveResponse(request.0, null_mut()) })?;
    within_budget(started)?;
    let mut status = 0u32;
    let mut size = std::mem::size_of_val(&status) as u32;
    // SAFETY: numeric status query writes exactly one DWORD to its sized output.
    success(unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            null(),
            (&mut status as *mut u32).cast(),
            &mut size,
            null_mut(),
        )
    })?;
    match status {
        200 => {}
        401 => return Err("E_USAGE_EXPIRED"),
        429 => return Err("E_USAGE_THROTTLED"),
        _ => return Err(UNAVAILABLE),
    }
    let mut body = Zeroizing::new(Vec::new());
    let mut chunk = Zeroizing::new([0u8; 8192]);
    loop {
        within_budget(started)?;
        let mut available = 0u32;
        // Query before reading: read only buffered data, never wait to fill an
        // arbitrary buffer while a server trickles bytes. An empty socket uses
        // the native receive timeout; the outer deadline bounds continuing data.
        success(unsafe { WinHttpQueryDataAvailable(request.0, &mut available) })?;
        within_budget(started)?;
        if available == 0 {
            return Ok(body);
        }
        if body.len().saturating_add(available as usize) > MAX_BODY {
            return Err(UNAVAILABLE);
        }
        let requested = available.min(chunk.len() as u32);
        let mut read = 0u32;
        // SAFETY: exact bounded mutable output, synchronous call, valid request.
        success(unsafe {
            WinHttpReadData(request.0, chunk.as_mut_ptr().cast(), requested, &mut read)
        })?;
        if read == 0 || read > requested {
            return Err(UNAVAILABLE);
        }
        body.extend_from_slice(&chunk[..read as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_fields_cannot_inject_or_overflow() {
        assert!(headers("synthetic.access.token", "synthetic-account_1").is_ok());
        for token in ["", "a\r\nHost: elsewhere", "a\0b", "a b", "é"] {
            assert!(headers(token, "synthetic").is_err());
        }
        for id in ["", "synthetic\r\nX: 1", "a:b", "a\0b", "é"] {
            assert!(headers("synthetic", id).is_err());
        }
        assert!(headers(&"x".repeat(65537), "synthetic").is_err());
        assert!(headers("synthetic", &"x".repeat(257)).is_err());
    }
}
