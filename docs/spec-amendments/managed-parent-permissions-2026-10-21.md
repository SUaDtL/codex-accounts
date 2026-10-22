# Existing Codex-managed permissions

On 2026-10-21 the owner explicitly approved the existing local CodexSandboxUsers read-only credential grant, then approved: "Support verified Codex-managed parent permissions" after the normal AppData package capability write grant was identified.

The Windows adapter supports that exact official package capability only on ancestor folders. The public identity derives from the fixed OpenAI.Codex_2p2nqsd0c76g0 package family and is independent of version numbers. Owners still follow existing owner rules. Vault and credential objects retain their private rules; arbitrary capabilities, unrelated readers and sandbox credential write grants remain refused. Existing Windows ACLs are never repaired or changed by this compatibility support.

The implementation uses a fixed public capability SID checked against Windows package derivation. A synthetic negative-control test accepts that ancestor grant and refuses it on strict vault/credential objects and refuses an unrelated capability. This amendment records owner-directed compatibility, not full real-account acceptance.
