# Security boundary

This bootstrap does not read or store authentication credentials. Its inventory
utility reads only nominated executable files and a bounded candidate configuration
file; arbitrary configuration values are never returned. Do not point it at a
credential file as an executable candidate.

No vault, key-store adapter, login or live switch exists yet. All installations
are unqualified. Library models and synthetic tests are not native safety proofs.
The future product's active file-mode credentials will remain plaintext; an
encrypted inactive vault will not defeat same-user malware or administrators.

Do not put tokens, account/workspace IDs, emails, login URLs, home/project paths,
raw helper output, `auth.json`, `cap_sid`, staging homes or vault files in GitHub
issues or commits. Default inventory output is minimized, but review even executable
hashes and configuration classifications before sharing them. Local-path mode is
not a sanitized diagnostic export.

Report a suspected code defect with a minimal synthetic reproduction. No private
vulnerability intake channel has been configured; do not assume that a public
issue is confidential. Do not disclose an active exploitable secret in an issue.

No third-party switcher implementation was copied into this source increment.
The supplied specification and research contain external citations, not executable
dependencies or audited security guarantees.
