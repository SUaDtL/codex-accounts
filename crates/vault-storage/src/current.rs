//! Saving a captured current account updates its generation and active protection
//! in the same encrypted registry commit. Callers supply only a native snapshot.
use super::*;
use crate::journal::GenerationRef;
use codex_accounts_vault::Identity;

impl<D: Files> Storage<D> {
    pub(crate) fn last_known_active_profile(&self) -> Result<Option<ProfileId>, StorageError> {
        if self.recovery() != Recovery::Clean {
            return Ok(None);
        }
        self.idle()?;
        if !self.registry.active_known {
            return Ok(None);
        }
        self.registry
            .active
            .map(ProfileId::from_bytes)
            .transpose()
            .map_err(StorageError::from)
    }

    pub(crate) fn rename_profile(
        &mut self,
        root: &RootKey,
        profile: ProfileId,
        label: ProfileText,
    ) -> Result<(), StorageError> {
        self.idle()?;
        let mut next = self.copy_registry()?;
        let selected = next
            .profiles
            .iter_mut()
            .find(|entry| entry.id == profile.to_bytes())
            .ok_or(StorageError::Missing)?;
        if selected.label.0.as_str() == label.0.as_str() {
            return Ok(());
        }
        selected.label = label;
        self.commit(root, next, vec![], vec![])
    }

    pub(crate) fn ensure_operation_idle(&self) -> Result<(), StorageError> {
        self.idle()
    }
    pub(crate) fn current_reference(
        &self,
        identity: &Identity,
    ) -> Result<GenerationRef, StorageError> {
        self.idle()?;
        let profile = self
            .registry
            .profiles
            .iter()
            .find(|p| &p.identity == identity)
            .ok_or(StorageError::Missing)?;
        Ok(GenerationRef {
            profile: profile.id,
            generation: profile.latest,
        })
    }

    pub(crate) fn save_current(
        &mut self,
        root: &RootKey,
        capture: Capture,
        label: Option<ProfileText>,
    ) -> Result<ProfileId, StorageError> {
        self.save_profile(root, capture, label, true)
    }

    pub(crate) fn save_login(
        &mut self,
        root: &RootKey,
        capture: Capture,
        label: ProfileText,
    ) -> Result<ProfileId, StorageError> {
        self.save_profile(root, capture, Some(label), false)
    }

    fn save_profile(
        &mut self,
        root: &RootKey,
        capture: Capture,
        label: Option<ProfileText>,
        mark_current: bool,
    ) -> Result<ProfileId, StorageError> {
        self.idle()?;
        let existing = self
            .registry
            .profiles
            .iter()
            .find(|p| p.identity == capture.identity);
        let (pid, parent) = match existing {
            Some(profile) => (profile.id, Some(profile.latest)),
            None => {
                if label.is_none() {
                    return Err(StorageError::Missing);
                }
                if self.registry.profiles.len() >= MAX_PROFILES {
                    return Err(StorageError::InputLimit);
                }
                let id = random_id()?;
                if self.registry.profiles.iter().any(|p| p.id == id) {
                    return Err(StorageError::AlreadyExists);
                }
                (id, None)
            }
        };
        let unchanged = if let Some(parent) = parent {
            let generation = self
                .registry
                .generations
                .iter()
                .find(|g| g.id == parent)
                .ok_or(StorageError::Corrupt)?;
            let rules: Vec<_> = generation
                .rules
                .iter()
                .map(|r| {
                    (
                        ResourceId::new(r.slot).expect("validated resource"),
                        if r.json {
                            ResourceShape::JsonObject
                        } else {
                            ResourceShape::Opaque
                        },
                        r.required,
                    )
                })
                .collect();
            let data = self.read_generation(root, generation)?;
            generation.schema == capture.schema
                && rules == capture.rules
                && rules.iter().all(|(slot, _, _)| {
                    data.resource(*slot).map(|r| r.as_bytes())
                        == capture.data.resource(*slot).map(|r| r.as_bytes())
                })
        } else {
            false
        };
        let mut next = self.copy_registry()?;
        let writes = if unchanged {
            vec![]
        } else {
            if self.registry.generations.len() >= MAX_GENERATIONS {
                return Err(StorageError::InputLimit);
            }
            let (generation, writes) = self.make_generation(root, pid, parent, &capture)?;
            let gid = generation.id;
            next.generations.push(generation);
            if let Some(profile) = next.profiles.iter_mut().find(|p| p.id == pid) {
                profile.latest = gid;
            } else {
                next.profiles.push(Profile {
                    id: pid,
                    identity: capture.identity,
                    label: label.ok_or(StorageError::Missing)?,
                    domain: ProfileText::new("ChatGPT".into())?,
                    latest: gid,
                    created: now()?,
                });
            }
            writes
        };
        // This association protects the account currently present in the selected
        // home. It is not an assertion of online acceptance or Desktop UI identity.
        if mark_current {
            next.active = Some(pid);
            next.active_known = true;
        }
        if !unchanged {
            next.rejected.retain(|id| *id != pid);
        }
        if !unchanged
            || (mark_current && (self.registry.active != Some(pid) || !self.registry.active_known))
        {
            self.commit(root, next, writes, vec![])?;
        }
        ProfileId::from_bytes(pid).map_err(StorageError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn capture(subject: &str, value: &[u8]) -> Capture {
        let slot = ResourceId::new(0).unwrap();
        Capture::new(
            Identity::new(
                "SYNTHETIC_ISSUER".into(),
                subject.into(),
                "SYNTHETIC_WORKSPACE".into(),
            )
            .unwrap(),
            1,
            vec![(slot, ResourceShape::JsonObject, true)],
            vec![Resource::present(slot, value.to_vec()).unwrap()],
        )
        .unwrap()
    }
    #[test]
    fn isolated_login_preserves_current_and_reuses_saved_identity() {
        let root = RootKey::generate().unwrap();
        let disk = super::super::tests::Memory::default();
        let mut store = Storage::create(disk.clone(), &root).unwrap();
        assert_eq!(store.last_known_active_profile().unwrap(), None);
        let current = store
            .save_current(
                &root,
                capture("A", b"{}"),
                Some(ProfileText::new("A".into()).unwrap()),
            )
            .unwrap();
        let saved = store
            .save_login(
                &root,
                capture("B", b"{\"synthetic\":1}"),
                ProfileText::new("B".into()).unwrap(),
            )
            .unwrap();
        assert_eq!(store.registry.active, Some(current.to_bytes()));
        assert!(store.registry.active_known);
        assert_eq!(store.last_known_active_profile().unwrap(), Some(current));
        let first = store.latest(saved).unwrap();
        assert_eq!(
            store
                .save_login(
                    &root,
                    capture("B", b"{\"synthetic\":2}"),
                    ProfileText::new("Ignored duplicate label".into()).unwrap()
                )
                .unwrap(),
            saved
        );
        assert_ne!(first, store.latest(saved).unwrap());
        drop(store);
        let mut store = Storage::open(disk, &root).unwrap();
        assert_eq!(store.last_known_active_profile().unwrap(), Some(current));
        store.registry.active_known = false;
        assert_eq!(store.last_known_active_profile().unwrap(), None);
        store.registry.active_known = true;
        store.blocked = true;
        assert_eq!(store.last_known_active_profile().unwrap(), None);
        store.blocked = false;
        assert_eq!(store.registry.active, Some(current.to_bytes()));
        assert_eq!(store.profiles().unwrap().len(), 2);
        assert_eq!(
            store
                .profiles()
                .unwrap()
                .into_iter()
                .find(|p| p.id == saved)
                .unwrap()
                .label,
            "B"
        );
        assert_eq!(
            store.remove(&root, current),
            Err(StorageError::ActiveProfile)
        );
        store.remove(&root, saved).unwrap();
    }
    #[test]
    fn current_capture_reuses_profile_updates_exact_bytes_and_protects_active() {
        let root = RootKey::generate().unwrap();
        let disk = super::super::tests::Memory::default();
        let mut store = Storage::create(disk.clone(), &root).unwrap();
        let first = store
            .save_current(
                &root,
                capture("A", b"{\"synthetic\":1}"),
                Some(ProfileText::new("A".into()).unwrap()),
            )
            .unwrap();
        let generation = store.latest(first).unwrap();
        assert_eq!(
            store
                .save_current(&root, capture("A", b"{\"synthetic\":1}"), None)
                .unwrap(),
            first
        );
        assert_eq!(store.latest(first).unwrap(), generation);
        store
            .save_current(&root, capture("A", b"{\"synthetic\":2}"), None)
            .unwrap();
        assert_ne!(store.latest(first).unwrap(), generation);
        assert_eq!(store.remove(&root, first), Err(StorageError::ActiveProfile));
        assert_eq!(
            store.save_current(&root, capture("B", b"{}"), None),
            Err(StorageError::Missing)
        );
        let second = store
            .save_current(
                &root,
                capture("B", b"{}"),
                Some(ProfileText::new("B".into()).unwrap()),
            )
            .unwrap();
        drop(store);
        let mut store = Storage::open(disk.clone(), &root).unwrap();
        assert_eq!(store.registry.active, Some(second.to_bytes()));
        assert_eq!(store.last_known_active_profile().unwrap(), Some(second));
        assert_eq!(store.profiles().unwrap().len(), 2);
        let generation = store.latest(second).unwrap();
        let count = store.registry.generations.len();
        let captured = store
            .registry
            .generations
            .iter()
            .find(|g| g.id == generation.to_bytes())
            .unwrap()
            .captured;
        store
            .rename_profile(&root, second, ProfileText::new("Renamed B".into()).unwrap())
            .unwrap();
        assert_eq!(store.latest(second).unwrap(), generation);
        assert_eq!(store.registry.generations.len(), count);
        assert_eq!(store.registry.active, Some(second.to_bytes()));
        assert_eq!(
            store
                .current_reference(&capture("B", b"{}").identity)
                .unwrap()
                .profile,
            second.to_bytes()
        );
        drop(store);
        let store = Storage::open(disk, &root).unwrap();
        let metadata = store
            .profiles()
            .unwrap()
            .into_iter()
            .find(|p| p.id == second)
            .unwrap();
        assert_eq!(metadata.label, "Renamed B");
        assert_eq!(metadata.captured_at, captured);
    }
}
