//! Pure decisions for the two fixed vault slots. Observations are not access
//! authority: the native adapter must still validate and retain every handle.
use crate::StorageError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotState {
    Absent,
    Directory,
    Unknown,
    AccessDenied,
    UnexpectedType,
    Changed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Open,
    Create,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    OpenLegacy,
    OpenProfile,
    CreateProfile,
}

pub(crate) fn decide(
    operation: Operation,
    legacy: SlotState,
    profile: SlotState,
) -> Result<Decision, StorageError> {
    // Neither an inaccessible slot nor a changed observation is absence. Check
    // both slots before selection; use legacy-first error ordering consistently.
    for state in [legacy, profile] {
        match state {
            SlotState::Unknown => return Err(StorageError::ActiveStateUnknown),
            SlotState::AccessDenied => return Err(StorageError::AccessDenied),
            SlotState::UnexpectedType => return Err(StorageError::UnsafePath),
            SlotState::Changed => return Err(StorageError::ExternalChange),
            SlotState::Absent | SlotState::Directory => {}
        }
    }
    match (operation, legacy, profile) {
        (Operation::Create, SlotState::Absent, SlotState::Absent) => Ok(Decision::CreateProfile),
        (Operation::Create, _, _) => Err(StorageError::AlreadyExists),
        (Operation::Open, SlotState::Directory, SlotState::Absent) => Ok(Decision::OpenLegacy),
        (Operation::Open, SlotState::Absent, SlotState::Directory) => Ok(Decision::OpenProfile),
        (Operation::Open, SlotState::Absent, SlotState::Absent) => Err(StorageError::Missing),
        (Operation::Open, _, _) => Err(StorageError::ActiveStateUnknown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use StorageError::*;

    const STATES: [SlotState; 6] = [
        SlotState::Absent,
        SlotState::Directory,
        SlotState::Unknown,
        SlotState::AccessDenied,
        SlotState::UnexpectedType,
        SlotState::Changed,
    ];
    type Outcome = Result<Decision, StorageError>;
    const UNKNOWN: Outcome = Err(ActiveStateUnknown);
    const DENIED: Outcome = Err(AccessDenied);
    const WRONG_TYPE: Outcome = Err(UnsafePath);
    const CHANGED: Outcome = Err(ExternalChange);
    const EXISTS: Outcome = Err(AlreadyExists);

    // Rows are legacy observations; columns are profile observations. These
    // explicit oracles cover all 72 inputs without reproducing the resolver.
    const OPEN: [[Outcome; 6]; 6] = [
        [
            Err(Missing),
            Ok(Decision::OpenProfile),
            UNKNOWN,
            DENIED,
            WRONG_TYPE,
            CHANGED,
        ],
        [
            Ok(Decision::OpenLegacy),
            UNKNOWN,
            UNKNOWN,
            DENIED,
            WRONG_TYPE,
            CHANGED,
        ],
        [UNKNOWN; 6],
        [DENIED; 6],
        [WRONG_TYPE; 6],
        [CHANGED; 6],
    ];
    const CREATE: [[Outcome; 6]; 6] = [
        [
            Ok(Decision::CreateProfile),
            EXISTS,
            UNKNOWN,
            DENIED,
            WRONG_TYPE,
            CHANGED,
        ],
        [EXISTS, EXISTS, UNKNOWN, DENIED, WRONG_TYPE, CHANGED],
        [UNKNOWN; 6],
        [DENIED; 6],
        [WRONG_TYPE; 6],
        [CHANGED; 6],
    ];

    #[test]
    fn exhaustive_slot_decision_table() {
        let mut checked = 0;
        for (operation, table) in [(Operation::Open, OPEN), (Operation::Create, CREATE)] {
            for (row, legacy) in STATES.into_iter().enumerate() {
                for (column, profile) in STATES.into_iter().enumerate() {
                    assert_eq!(
                        decide(operation, legacy, profile),
                        table[row][column],
                        "operation={operation:?}, legacy={legacy:?}, profile={profile:?}"
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 72);
    }

    #[test]
    fn uncertain_slots_never_become_absence_or_selection() {
        for (state, error) in [
            (SlotState::Unknown, ActiveStateUnknown),
            (SlotState::AccessDenied, AccessDenied),
            (SlotState::UnexpectedType, UnsafePath),
            (SlotState::Changed, ExternalChange),
        ] {
            for operation in [Operation::Open, Operation::Create] {
                for other in [SlotState::Absent, SlotState::Directory] {
                    assert_eq!(decide(operation, state, other), Err(error));
                    assert_eq!(decide(operation, other, state), Err(error));
                }
            }
        }
    }

    #[test]
    fn opening_requires_exactly_one_unambiguous_existing_slot() {
        assert_eq!(
            decide(Operation::Open, SlotState::Directory, SlotState::Directory),
            Err(ActiveStateUnknown)
        );
        assert_eq!(
            decide(Operation::Open, SlotState::Absent, SlotState::Absent),
            Err(Missing)
        );
        assert_eq!(
            decide(Operation::Open, SlotState::Directory, SlotState::Absent),
            Ok(Decision::OpenLegacy)
        );
        assert_eq!(
            decide(Operation::Open, SlotState::Absent, SlotState::Directory),
            Ok(Decision::OpenProfile)
        );
    }

    #[test]
    fn creation_refuses_every_existing_slot() {
        for (legacy, profile) in [
            (SlotState::Directory, SlotState::Absent),
            (SlotState::Absent, SlotState::Directory),
            (SlotState::Directory, SlotState::Directory),
        ] {
            assert_eq!(
                decide(Operation::Create, legacy, profile),
                Err(AlreadyExists)
            );
        }
        assert_eq!(
            decide(Operation::Create, SlotState::Absent, SlotState::Absent),
            Ok(Decision::CreateProfile)
        );
    }
}
