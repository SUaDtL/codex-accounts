# Explicit registered image observation

This Q0 increment follows merged PR #16 at
`9389c33b7bc8d41fee92f14046eebd546c5c86f2`. Its purpose is to inspect one
explicitly nominated executable inside the selected registered package, including
an image that is not declared as an Application in the package manifest.
F-004, F-008 and S-010 still require a separately verified runtime binding before
execution. This observation creates no such binding.

## Read-only contract

`--registered-image` requires both an opaque `--candidate` ID and
`--registered-metadata`. It accepts at most one validated package-relative `.exe`.
Absolute, parent-relative, device, UNC and alternate-stream paths refuse. The
existing home/runtime flag conflicts remain refusals. Configuration, auth and
history files cannot be nominated through this interface.

The collector uses the held registered root and retains internal directories,
manifest, declared images and nominated image through final registration and
identity revalidation. The image must pass the existing bounded PE/hash/sharing
and link checks. A failure refuses the observation, rather than selecting a
different candidate or treating uncertainty as absence. No process is launched.

Declared images remain in their original array. The optional `nominated_image`
has provenance `caller_nominated_package_relative` and role
`unqualified_nominated_image`. Its hash and cached-signature observation do not
establish publisher authority or runtime role. Outer ancestry, modification
protection and effective home/backend/policy remain unknown. The compatibility
catalog and mutation authority stay disabled.

Default reports, errors and Debug output omit raw image paths. The existing
terminal-only local display remains separately guarded and must not be uploaded.

## Verification

The Rust library and actual diagnostic binary exercise valid and invalid flag
combinations and redaction. Real Windows fixtures exercise a nonmanifest image,
retained handles, exact bytes, exclusive sensitive-file canaries, registration
changes, unsafe leaves and junction traversal without reading the external target.
All prior metadata and BOM assertions remain required.

CI adds the three new named behaviors to the existing inventory: 54 native cases
in each profile, including all 51 predecessor identities. The actual compiled
inventory must prove the complete/disjoint workspace complement. Two-worker
ordinary execution, serial outer restart suites, deadlines and no-retry behavior
remain unchanged. Final source checks, independent review and hosted evidence
belong to the new PR; PR #16's green results are historical.

## Remaining integration

Read-only collection on 2026-10-15 succeeded against the selected registered
Windows x64 package `26.1007.9446.0`. Its two declared images remained separate;
the explicitly nominated image was 334,121,776 bytes with SHA-256
`e98c516f1c8935789aabe9122ee7e05ec4c3efd1ba6acf179242e031c495b99e`.
The report retained stable read identities and registration, reported cached
signature acceptance as an observation, and left role/publisher/home/backend/
policy/ancestry unqualified. No process was launched and no user configuration,
credentials or history were read. No CLI version or official
schema is inferred from that hash.

A file named `codex.exe` and a successful metadata observation are insufficient
to approve execution. Official publisher/runtime binding, exact schema, effective
home/backend/policy, auth-resource semantics and lifecycle observations remain Q0
work. Native pipes, environment policy, helper ownership and recovery integration
follow their reviewed dependencies. This increment neither reads real auth nor
closes any complete Q0-Q5 or release scenario.
