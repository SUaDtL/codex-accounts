# Why the security model works this way

Codex Accounts stores inactive account credentials in an encrypted local vault.
Windows DPAPI binds the vault key to the signed-in Windows user. Every vault file,
temporary login home and switching file is also checked for its owner, permissions,
links and physical storage location before the app trusts it.

The active Codex credential is different. Codex Desktop expects `auth.json` in its
normal home, so the selected account must remain in that format while Codex uses it.
The switcher cannot encrypt that live file without making Codex unable to read it.
Closing the vault protects saved inactive accounts. It does not protect the current
account from another process already running as the same Windows user.

The app delegates sign-in and credential acceptance to the installed official Codex
runtime. It does not implement OAuth token refresh and does not expose credentials
through a command line, API or plugin surface. The app validates the official Store
package, publisher registration, Authenticode signature, executable architecture and
file protections before it lets that runtime participate in an account operation. The
Desktop and bundled runtime must present the same Windows-validated signing certificate.
The browser authorize request must also match the reviewed Codex client, PKCE, scope and
loopback callback contract before the app opens it.

Switching is journaled because several files and processes must agree on one account.
If power loss or a crash interrupts the change, the journal lets the app finish or
restore the recorded operation without guessing. Plaintext staging exists only inside
Codex's protected home while an operation is active or awaiting recovery, because the
official runtime ultimately needs the same plaintext credential file.

An optional hardened vault adds a strong passphrase around the DPAPI-protected root
key. This protects a copied vault file from offline guessing and still keeps the vault
bound to its Windows user. A short interface PIN is only a privacy lock for an open
application window. It is not encryption and does not defend against same-user malware.

These limits are deliberate. The project does not add cloud sync, an updater, direct
OAuth refresh, credential export, traffic proxying, forced process termination or an
exact Codex version allowlist. Each would either expand credential exposure or make the
app brittle without improving the trust boundary.
