# Desktop shell boundary (Q4)

The intended product shell is Tauri 2 with bundled HTML/CSS and minimal TypeScript,
per SPEC-DECISIONS. It is not implemented or a workspace member in this increment.
There is no fabricated package.json, downloaded WebView, GUI dependency tree,
launcher or account button wired to unqualified OS actions.

Introduce exact reviewed Tauri/npm pins and generated transitive lockfiles when
adding Q4. Their absence is a staged supply-chain gap against S-015, not a completed
S-015 acceptance claim. Keep secrets, raw paths, shell strings and arbitrary RPC
out of the renderer. Native consent and every backend check remain mandatory.
