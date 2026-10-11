# Security boundary

The discovery utility reads registered-package metadata, nominated executable
files and a bounded candidate configuration file. It does not open live auth,
credential stores or history; arbitrary configuration values are never returned.
Do not point it at a credential file as an executable candidate.

Development libraries implement authenticated encrypted vault storage, Windows
current-user DPAPI, recovery journals and private native lifecycle/resource
primitives. Their native tests use owned synthetic fixtures. Runtime sequencing
and transport construction remain private or test-only, and the static UI's
operational actions are disabled. No production login, live account switch or
qualified Desktop adapter exists. All installations remain unqualified.
The catalog is empty and production credential authority is disabled. A model
result or native fixture does not establish the real Desktop contract.
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
