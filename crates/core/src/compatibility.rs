//! Discovery facts are observations, never capabilities to mutate an account.
//! No deserializer can construct a qualified installation in this module.
use crate::{Backend, ErrorCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observation<T> {
    Observed(T),
    Absent,
    Inaccessible,
    Conflicting,
    Unsupported,
    Unknown,
}

impl<T> Observation<T> {
    pub const fn state(&self) -> &'static str {
        match self {
            Self::Observed(_) => "observed",
            Self::Absent => "absent",
            Self::Inaccessible => "inaccessible",
            Self::Conflicting => "conflicting",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendBasis {
    ExplicitDeclaration,
    ExactBuildDefault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    Allowed,
    Denied,
}

/// A pure evaluation input. Even all-positive inputs never grant authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub declared_backend: Observation<Backend>,
    pub effective_backend: Observation<(Backend, BackendBasis)>,
    pub home: Observation<()>,
    pub policy: Observation<Policy>,
    pub exact_build_rule: Observation<()>,
    pub override_present: bool,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            declared_backend: Observation::Unknown,
            effective_backend: Observation::Unknown,
            home: Observation::Unknown,
            policy: Observation::Unknown,
            exact_build_rule: Observation::Unknown,
            override_present: false,
        }
    }
}

impl Context {
    pub fn refusal(&self) -> ErrorCode {
        if self.policy == Observation::Observed(Policy::Denied) {
            return ErrorCode::PolicyDenied;
        }
        if matches!(self.effective_backend, Observation::Observed((backend, _))
            if !matches!(backend, Backend::File | Backend::Unknown))
        {
            return ErrorCode::BackendUnsupported;
        }
        // A declared value does not become an effective value here. Incomplete
        // policy has an unresolved observation, not fabricated evidence of denial.
        ErrorCode::CompatUnknown
    }

    pub const fn credential_mutation_enabled(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError {
    NoMatchingRegisteredCandidate,
    SelectionRequired,
    CandidateDisappeared,
    AmbiguousId,
}

/// IDs are local opaque identifiers, not executable names or paths.
pub fn select(ids: &[String], requested: Option<&str>) -> Result<usize, SelectionError> {
    if ids.is_empty() {
        return Err(SelectionError::NoMatchingRegisteredCandidate);
    }
    let requested = requested.ok_or(SelectionError::SelectionRequired)?;
    let mut found = ids.iter().enumerate().filter(|(_, id)| id.as_str() == requested);
    let index = found.next().ok_or(SelectionError::CandidateDisappeared)?.0;
    if found.next().is_some() {
        return Err(SelectionError::AmbiguousId);
    }
    Ok(index)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureObservation {
    CachedTrustAccepted,
    NotEstablished,
}

/// Matching a publisher string or a cached signature is not enough. This pure
/// comparison is used only to describe disagreements, never to launch a process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityFacts<'a> {
    pub publisher: &'a str,
    pub package: &'a str,
    pub architecture: &'a str,
    pub desktop_hash: &'a str,
    pub runtime_hash: &'a str,
    pub runtime_in_package: bool,
    pub signature: SignatureObservation,
}

pub fn same_observed_identity(a: &IdentityFacts<'_>, b: &IdentityFacts<'_>) -> bool {
    a == b && a.runtime_in_package
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_candidate_single_and_multiple_require_explicit_selection() {
        assert_eq!(select(&[], None), Err(SelectionError::NoMatchingRegisteredCandidate));
        let one = vec!["synthetic-a".to_owned()];
        assert_eq!(select(&one, None), Err(SelectionError::SelectionRequired));
        assert_eq!(select(&one, Some("synthetic-a")), Ok(0));
        let two = vec!["synthetic-a".to_owned(), "synthetic-b".to_owned()];
        assert_eq!(select(&two, None), Err(SelectionError::SelectionRequired));
        assert_eq!(select(&two, Some("synthetic-b")), Ok(1));
        assert_eq!(select(&two, Some("missing")), Err(SelectionError::CandidateDisappeared));
        assert_eq!(select(&["x".into(), "x".into()], Some("x")), Err(SelectionError::AmbiguousId));
    }

    #[test]
    fn every_observation_state_is_distinct() {
        let facts = [Observation::Observed(()), Observation::Absent, Observation::Inaccessible,
            Observation::Conflicting, Observation::Unsupported, Observation::Unknown];
        let names: std::collections::BTreeSet<_> = facts.iter().map(Observation::state).collect();
        assert_eq!(names.len(), facts.len());
    }

    #[test]
    fn declarations_missing_settings_and_shell_context_never_establish_effective_state() {
        for declaration in [Observation::Observed(Backend::File), Observation::Observed(Backend::Auto),
            Observation::Observed(Backend::Keyring), Observation::Absent, Observation::Inaccessible,
            Observation::Conflicting, Observation::Unknown] {
            let c = Context { declared_backend: declaration, override_present: true, ..Context::default() };
            assert_eq!(c.effective_backend, Observation::Unknown);
            assert_eq!(c.home, Observation::Unknown);
            assert_eq!(c.policy, Observation::Unknown);
            assert_eq!(c.refusal(), ErrorCode::CompatUnknown);
            assert!(!c.credential_mutation_enabled());
        }
    }

    #[test]
    fn complete_denial_is_distinct_from_incomplete_policy() {
        let mut c = Context { policy: Observation::Inaccessible, ..Context::default() };
        assert_eq!(c.refusal(), ErrorCode::CompatUnknown);
        c.policy = Observation::Observed(Policy::Denied);
        assert_eq!(c.refusal(), ErrorCode::PolicyDenied);
    }

    #[test]
    fn even_all_positive_fixture_inputs_are_not_authority() {
        for basis in [BackendBasis::ExplicitDeclaration, BackendBasis::ExactBuildDefault] {
            for backend in [Backend::File, Backend::Auto, Backend::Keyring, Backend::Secrets,
                Backend::Ephemeral, Backend::Unknown] {
                let c = Context { declared_backend: Observation::Observed(backend),
                    effective_backend: Observation::Observed((backend, basis)),
                    home: Observation::Observed(()), policy: Observation::Observed(Policy::Allowed),
                    exact_build_rule: Observation::Observed(()), override_present: false };
                assert!(!c.credential_mutation_enabled());
                assert!(crate::mutation_availability(backend).is_err());
            }
        }
    }

    #[test]
    fn replacements_and_detached_runtime_change_observed_identity() {
        let a = IdentityFacts { publisher: "SYNTHETIC publisher", package: "SYNTHETIC package",
            architecture: "x64", desktop_hash: "synthetic-digest-a", runtime_hash: "synthetic-digest-b",
            runtime_in_package: true, signature: SignatureObservation::CachedTrustAccepted };
        assert!(same_observed_identity(&a, &a));
        for b in [IdentityFacts { publisher: "OTHER publisher", ..a },
            IdentityFacts { architecture: "arm64", ..a },
            IdentityFacts { desktop_hash: "changed", ..a },
            IdentityFacts { runtime_hash: "changed", ..a },
            IdentityFacts { runtime_in_package: false, ..a },
            IdentityFacts { signature: SignatureObservation::NotEstablished, ..a }] {
            assert!(!same_observed_identity(&a, &b));
        }
    }
}
