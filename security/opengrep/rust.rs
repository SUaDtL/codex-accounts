fn fixtures() {
    // ruleid: rust-tls-verification-disabled
    client.danger_accept_invalid_certs(true);
    // ok: rust-tls-verification-disabled
    client.danger_accept_invalid_certs(false);
    // ruleid: native-tls-ignore-flags
    let flags = SECURITY_FLAG_IGNORE_UNKNOWN_CA;
    // ok: native-tls-ignore-flags
    let flags = WINHTTP_FLAG_SECURE;
    // ruleid: rust-shell-command
    Command::new("cmd.exe");
    // ok: rust-shell-command
    Command::new(verified_executable);
}
