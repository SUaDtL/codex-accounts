# CA-04D shared-interface review for macOS preparation

Source assessment in PR #13, not owner approval or native qualification.
Predecessor: CA-04C 455baa2eeee614586b26d419ee961475fe1ad23b.
Reviewed source: CA-04D 345533eacd4ed4ed288fa7a4580dd4e408106ede.
The original seven-file baseline is retained verbatim in interface-baseline-ca04c.json.

## Compared changes and consequences

- coordinator.rs: write_journal retains the greatest existing format and selects
  CAREG003 only for staging evidence. Live and staging archives are distinguished;
  subset reuse applies only to byte-identical staged slots in an authenticated
  batch after partial deletion. Live conflict equality remains whole-set equality.
  Generation holds and newest source/target references are unchanged.
- journal.rs: EvidenceKind is authenticated registry data. Staging evidence requires
  format 3, a present blob and a slot in the recorded staging/restoration mask.
  Evidence cannot become a credential generation or a Desktop confirmation.
- stage_repair.rs (new eighth baseline file): private StagingRepair callback must
  keep validated objects exclusively pinned until encrypted commit/readback succeeds.
  Confirmation, lock, recovery, quiescence, key, live-state and write guards precede
  repair. The durable intent leaves Recovery/SwitchPending; restoration is separate.
  Terminal, committed, uncaptured-helper and unregistered states refuse repair.
- core, platform, root-key, storage engine and lifecycle model bytes are unchanged.

A macOS implementation cannot simply translate the Windows exclusive-sharing
assumption into flock: its advisory behavior does not provide that exclusion.
Keep repair unavailable until a separately reviewed descriptor/namespace design
establishes the callback's preconditions or explicitly refuses unsupported cases.
File flush, directory metadata and physical-power-loss evidence remain separate.

The new v2 baseline records this reviewed source delta and preserves the original
record and review hashes. Its check is source consistency only. All macOS native
cases remain NOT RUN; native coding still needs owner selection, exact SDK/binding
review, an authorized test host and review of CA-04D. No qualification flag,
production constructor, Keychain access, credential path or Windows-derived pass
is introduced. A changed source or review requires another visible scoped review.
