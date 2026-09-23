#![forbid(unsafe_code)]
use crate::{codec, records::*, Capture, GenerationId, ProfileId, ProfileText, StorageError};
use codex_accounts_vault::{CredentialSet, Resource, ResourceId, ResourceShape};
use codex_accounts_vault_crypto::{Envelope, EnvelopeContext, Purpose, RootKey};
use std::collections::BTreeSet;

/// Private boundary; no public custom filesystem, root key, or qualification injection.
/// stage creates without overwriting; publish compares both expected old and new
/// bytes; erase uses a pinned, validated object, never an unchecked name deletion.
type EncryptedWrites = Vec<(Blob, Vec<u8>)>;

pub(crate) trait Files {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>, StorageError>;
    fn stage(&mut self, name: &str, bytes: &[u8]) -> Result<(), StorageError>;
    fn publish(
        &mut self,
        name: &str,
        expected: Option<&[u8]>,
        staged: &[u8],
    ) -> Result<(), StorageError>;
    fn erase(&mut self, name: &str, expected: &[u8]) -> Result<(), StorageError>;
    fn list(&self) -> Result<Vec<String>, StorageError>;
}
fn control_context(root: &RootKey) -> Result<EnvelopeContext, StorageError> {
    // Fixed root-bound control slot. Registry/generation contexts are selected
    // through its authenticated references, never from their envelope headers.
    Ok(EnvelopeContext::new(
        Purpose::Journal,
        1,
        root.identifier(),
        root.identifier(),
        0,
    )?)
}
fn open_state(root: &RootKey, bytes: &[u8]) -> Result<State, StorageError> {
    let data = root.open(&control_context(root)?, &Envelope::from_bytes(bytes)?)?;
    codec::read_state(data.expose())
}

pub(crate) struct Storage<D: Files> {
    pub(crate) disk: D,
    state: State,
    state_bytes: Option<Vec<u8>>,
    registry: Registry,
    later_startup: bool,
    blocked: bool,
}
impl<D: Files> Storage<D> {
    pub fn create(disk: D, root: &RootKey) -> Result<Self, StorageError> {
        if disk.list()?.iter().any(|n| n != "key.cakp" && n != "lock") {
            return Err(StorageError::AlreadyExists);
        }
        let mut store = Self {
            disk,
            state: State::initial(),
            state_bytes: None,
            registry: Registry::empty(),
            later_startup: false,
            blocked: false,
        };
        store.commit(root, Registry::empty(), vec![], vec![])?;
        Ok(store)
    }
    pub fn open(mut disk: D, root: &RootKey) -> Result<Self, StorageError> {
        let mut current = disk.read("state.bin")?;
        if let Some(stage) = disk.read("state.bin.stage")? {
            let next = open_state(root, &stage).map_err(|_| StorageError::RecoveryRequired)?;
            let previous = current
                .as_deref()
                .map(|b| open_state(root, b))
                .transpose()?;
            if next.parent != current.as_deref().map(digest)
                || next.sequence
                    != previous
                        .as_ref()
                        .map_or(Some(1), |s| s.sequence.checked_add(1))
                        .ok_or(StorageError::Corrupt)?
            {
                return Err(StorageError::ExternalChange);
            }
            if let Some(reference) = &next.current {
                let selected = Self::load_registry(&disk, root, reference)?;
                Self::verify_registry_on(&disk, root, &selected)?;
            }
            disk.publish("state.bin", current.as_deref(), &stage)?;
            current = Some(stage);
        }
        let bytes = current.ok_or(StorageError::RecoveryRequired)?;
        let state = open_state(root, &bytes)?;
        let registry = match &state.current {
            Some(b) => Self::load_registry(&disk, root, b)?,
            None if state.next.is_some() => Registry::empty(),
            None => return Err(StorageError::Corrupt),
        };
        let store = Self {
            disk,
            state,
            state_bytes: Some(bytes),
            registry,
            later_startup: true,
            blocked: false,
        };
        store.verify_registry(root, &store.registry)?;
        store.inventory()?;
        Ok(store)
    }
    fn fresh(&self) -> Result<(), StorageError> {
        if self.blocked {
            return Err(StorageError::RecoveryRequired);
        }
        if self.disk.read("state.bin")? != self.state_bytes
            || self.disk.read("state.bin.stage")?.is_some()
        {
            return Err(StorageError::ExternalChange);
        }
        Ok(())
    }
    fn ready(&self) -> Result<(), StorageError> {
        self.fresh()?;
        if self.state.next.is_some() || self.state.deleting {
            return Err(StorageError::RecoveryRequired);
        }
        Ok(())
    }
    pub fn recovery(&self) -> Recovery {
        if self.blocked {
            Recovery::Blocked
        } else if self.state.next.is_some() {
            Recovery::CommitPending
        } else if self.state.deleting {
            Recovery::CleanupPending
        } else {
            Recovery::Clean
        }
    }
    fn inventory(&self) -> Result<(), StorageError> {
        let mut allowed: BTreeSet<String> = ["lock", "key.cakp", "state.bin"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        for b in self
            .registry
            .blobs()
            .iter()
            .chain(self.state.current.iter())
            .chain(&self.state.writes)
            .chain(&self.state.garbage)
        {
            allowed.insert(b.key.name());
            allowed.insert(format!("{}.stage", b.key.name()));
        }
        let actual = self.disk.list()?;
        if actual.len() > MAX_BLOBS * 2 + 3 || actual.iter().any(|n| !allowed.contains(n)) {
            return Err(StorageError::ExternalChange);
        }
        Ok(())
    }
    fn change_state(&mut self, root: &RootKey, mut next: State) -> Result<(), StorageError> {
        self.fresh()?;
        next.sequence = self
            .state
            .sequence
            .checked_add(1)
            .ok_or(StorageError::InputLimit)?;
        next.parent = self.state_bytes.as_deref().map(digest);
        let bytes = root.seal(&control_context(root)?, &codec::state(&next)?)?;
        let bytes = bytes.as_bytes();
        // Even a failed flush/rename may have taken effect. Poison this instance
        // before the first effect; reopening reconciles the actual durable bytes.
        self.blocked = true;
        self.disk.stage("state.bin", bytes)?;
        self.disk
            .publish("state.bin", self.state_bytes.as_deref(), bytes)?;
        self.state_bytes = Some(bytes.to_vec());
        self.state = next;
        self.blocked = false;
        Ok(())
    }
    fn read_blob(disk: &D, b: &Blob) -> Result<Vec<u8>, StorageError> {
        let bytes = disk
            .read(&b.key.name())?
            .ok_or(StorageError::RecoveryRequired)?;
        b.check(&bytes)?;
        Ok(bytes)
    }
    fn load_registry(disk: &D, root: &RootKey, b: &Blob) -> Result<Registry, StorageError> {
        if b.key.kind != 1 || b.key.profile != root.identifier() {
            return Err(StorageError::Corrupt);
        }
        let bytes = Self::read_blob(disk, b)?;
        let data = root.open(&b.key.context()?, &Envelope::from_bytes(&bytes)?)?;
        codec::read_registry(data.expose())
    }
    fn verify_registry(&self, root: &RootKey, r: &Registry) -> Result<(), StorageError> {
        Self::verify_registry_on(&self.disk, root, r)
    }
    fn verify_registry_on(disk: &D, root: &RootKey, r: &Registry) -> Result<(), StorageError> {
        r.validate()?;
        for g in &r.generations {
            let p = r
                .profiles
                .iter()
                .find(|p| p.id == g.profile)
                .ok_or(StorageError::Corrupt)?;
            let bytes = Self::read_blob(disk, &g.manifest)?;
            let data = root.open(&g.manifest.key.context()?, &Envelope::from_bytes(&bytes)?)?;
            if data.expose() != codec::generation(g, &p.identity)?.as_slice() {
                return Err(StorageError::Corrupt);
            }
            Self::read_generation_on(disk, root, g)?;
        }
        Ok(())
    }
    fn read_generation(
        &self,
        root: &RootKey,
        g: &Generation,
    ) -> Result<CredentialSet, StorageError> {
        Self::read_generation_on(&self.disk, root, g)
    }
    fn read_generation_on(
        disk: &D,
        root: &RootKey,
        g: &Generation,
    ) -> Result<CredentialSet, StorageError> {
        let mut rules = vec![];
        let mut resources = vec![];
        for r in &g.rules {
            let id = ResourceId::new(r.slot)?;
            rules.push((
                id,
                if r.json {
                    ResourceShape::JsonObject
                } else {
                    ResourceShape::Opaque
                },
                r.required,
            ));
            resources.push(match &r.blob {
                None => Resource::absent(id),
                Some(b) => {
                    let bytes = Self::read_blob(disk, b)?;
                    let data = root.open(&b.key.context()?, &Envelope::from_bytes(&bytes)?)?;
                    Resource::present(id, data.expose().to_vec())?
                }
            });
        }
        Ok(CredentialSet::new(&rules, resources)?)
    }
    fn put(&mut self, b: &Blob, bytes: &[u8]) -> Result<(), StorageError> {
        b.check(bytes)?;
        if let Some(existing) = self.disk.read(&b.key.name())? {
            b.check(&existing)?;
            return Ok(());
        }
        self.disk.stage(&b.key.name(), bytes)?;
        self.disk.publish(&b.key.name(), None, bytes)?;
        b.check(&Self::read_blob(&self.disk, b)?)
    }
    fn commit(
        &mut self,
        root: &RootKey,
        mut registry: Registry,
        mut writes: Vec<(Blob, Vec<u8>)>,
        retire: Vec<Blob>,
    ) -> Result<(), StorageError> {
        self.ready()?;
        registry.revision = self
            .registry
            .revision
            .checked_add(1)
            .ok_or(StorageError::InputLimit)?;
        let key = Key::registry(root, random_id()?);
        let ciphertext = root.seal(&key.context()?, &codec::registry(&registry)?)?;
        let target = Blob::new(key, ciphertext.as_bytes())?;
        writes.push((target.clone(), ciphertext.as_bytes().to_vec()));
        let mut next = self.state.clone();
        next.next = Some(target.clone());
        next.writes = writes.iter().map(|(b, _)| b.clone()).collect();
        next.inline = writes
            .iter()
            .filter(|(b, _)| b.key.kind != 3)
            .cloned()
            .collect();
        next.garbage.extend(retire);
        next.validate()?;
        self.change_state(root, next)?;
        self.blocked = true;
        for (b, bytes) in &writes {
            self.put(b, bytes)?;
        }
        self.verify_registry(root, &registry)?;
        self.blocked = false;
        self.finish(root, registry)
    }
    fn finish(&mut self, root: &RootKey, registry: Registry) -> Result<(), StorageError> {
        let mut clean = self.state.clone();
        clean.current = clean.next.take();
        if let Some(old) = self.state.current.clone() {
            clean.garbage.push(old);
        }
        clean.writes.clear();
        clean.inline.clear();
        self.change_state(root, clean)?;
        self.registry = registry;
        self.later_startup = false;
        Ok(())
    }
    pub fn reconcile(&mut self, root: &RootKey) -> Result<(), StorageError> {
        self.fresh()?;
        if self.state.deleting {
            return self.cleanup(root);
        }
        let Some(next) = self.state.next.clone() else {
            return Ok(());
        };
        let pending = self.state.clone();
        self.blocked = true;
        for b in &pending.writes {
            if let Some(data) = self.disk.read(&b.key.name())? {
                b.check(&data)?;
                continue;
            }
            let staged = self.disk.read(&format!("{}.stage", b.key.name()))?;
            if let Some(data) = staged {
                if b.check(&data).is_ok() {
                    self.disk.publish(&b.key.name(), None, &data)?;
                } else if let Some((_, complete)) = pending.inline.iter().find(|(v, _)| v == b) {
                    // Only authenticated inline METADATA can reconstruct a torn
                    // prefix. Resource payloads are never guessed or regenerated.
                    if data.len() >= complete.len() || !complete.starts_with(&data) {
                        return Err(StorageError::ExternalChange);
                    }
                    self.disk.erase(&format!("{}.stage", b.key.name()), &data)?;
                    self.put(b, complete)?;
                } else {
                    return Err(StorageError::ExternalChange);
                }
            } else if let Some((_, data)) = pending.inline.iter().find(|(v, _)| v == b) {
                self.put(b, data)?;
            } else {
                return Err(StorageError::RecoveryRequired);
            }
        }
        let registry = Self::load_registry(&self.disk, root, &next)?;
        if registry.revision
            != self
                .registry
                .revision
                .checked_add(1)
                .ok_or(StorageError::Corrupt)?
        {
            return Err(StorageError::Corrupt);
        }
        self.verify_registry(root, &registry)?;
        self.blocked = false;
        self.finish(root, registry)
    }
    /// Explicit storage-only rollback. Partial/foreign/corrupt staged bytes are
    /// preserved and block this operation, rather than silently deleting evidence.
    pub fn restore_previous(&mut self, root: &RootKey) -> Result<(), StorageError> {
        self.fresh()?;
        if self.state.next.is_none() || self.state.current.is_none() {
            return Err(StorageError::RecoveryRequired);
        }
        self.verify_registry(root, &self.registry)?;
        let reachable: BTreeSet<_> = self
            .registry
            .blobs()
            .iter()
            .map(|b| b.key)
            .chain(self.state.current.iter().map(|b| b.key))
            .collect();
        let mut clean = self.state.clone();
        clean.garbage.retain(|b| !reachable.contains(&b.key));
        for b in &self.state.writes {
            for n in [b.key.name(), format!("{}.stage", b.key.name())] {
                if let Some(bytes) = self.disk.read(&n)? {
                    b.check(&bytes)?;
                }
            }
            if !reachable.contains(&b.key) {
                clean.garbage.push(b.clone());
            }
        }
        clean.next = None;
        clean.writes.clear();
        clean.inline.clear();
        self.change_state(root, clean)?;
        self.later_startup = false;
        Ok(())
    }
    fn copy_registry(&self) -> Result<Registry, StorageError> {
        codec::read_registry(&codec::registry(&self.registry)?)
    }
    fn make_generation(
        &self,
        root: &RootKey,
        profile: Id,
        parent: Option<Id>,
        capture: &Capture,
    ) -> Result<(Generation, EncryptedWrites), StorageError> {
        let id = random_id()?;
        if self.registry.generations.iter().any(|g| g.id == id) {
            return Err(StorageError::AlreadyExists);
        }
        let mut writes = vec![];
        let mut rules = vec![];
        for slot in 0..16 {
            let rid = ResourceId::new(slot)?;
            let Some((_, shape, required)) = capture.rules.iter().find(|r| r.0 == rid) else {
                continue;
            };
            let bytes = capture
                .data
                .resource(rid)
                .ok_or(StorageError::InvalidData)?
                .as_bytes();
            let blob = if let Some(bytes) = bytes {
                let key = Key::resource(profile, id, slot);
                let sealed = root.seal(&key.context()?, bytes)?;
                let b = Blob::new(key, sealed.as_bytes())?;
                writes.push((b.clone(), sealed.as_bytes().to_vec()));
                Some(b)
            } else {
                None
            };
            rules.push(Rule {
                slot,
                json: *shape == ResourceShape::JsonObject,
                required: *required,
                blob,
            });
        }
        let key = Key::generation(profile, id);
        let mut g = Generation {
            profile,
            id,
            parent,
            schema: capture.schema,
            captured: now()?,
            rules,
            manifest: Blob {
                key,
                size: 0,
                hash: [0; 32],
            },
        };
        let sealed = root.seal(&key.context()?, &codec::generation(&g, &capture.identity)?)?;
        g.manifest = Blob::new(key, sealed.as_bytes())?;
        writes.push((g.manifest.clone(), sealed.as_bytes().to_vec()));
        Ok((g, writes))
    }
    pub fn add(
        &mut self,
        root: &RootKey,
        label: ProfileText,
        domain: ProfileText,
        capture: Capture,
    ) -> Result<(ProfileId, GenerationId), StorageError> {
        self.ready()?;
        if self.registry.profiles.len() >= MAX_PROFILES
            || self.registry.generations.len() >= MAX_GENERATIONS
        {
            return Err(StorageError::InputLimit);
        }
        if self
            .registry
            .profiles
            .iter()
            .any(|p| p.identity == capture.identity)
        {
            return Err(StorageError::IdentityMismatch);
        }
        let pid = random_id()?;
        if self.registry.profiles.iter().any(|p| p.id == pid) {
            return Err(StorageError::AlreadyExists);
        }
        let (g, writes) = self.make_generation(root, pid, None, &capture)?;
        let gid = g.id;
        let mut next = self.copy_registry()?;
        next.generations.push(g);
        next.profiles.push(Profile {
            id: pid,
            label,
            domain,
            identity: capture.identity,
            latest: gid,
            created: now()?,
        });
        self.commit(root, next, writes, vec![])?;
        Ok((ProfileId::from_bytes(pid)?, GenerationId::from_bytes(gid)?))
    }
    pub fn append(
        &mut self,
        root: &RootKey,
        profile: ProfileId,
        expected: GenerationId,
        capture: Capture,
    ) -> Result<GenerationId, StorageError> {
        self.ready()?;
        let pid = profile.to_bytes();
        let parent = expected.to_bytes();
        let p = self
            .registry
            .profiles
            .iter()
            .find(|p| p.id == pid)
            .ok_or(StorageError::Missing)?;
        if p.latest != parent {
            return Err(StorageError::StaleParent);
        }
        if p.identity != capture.identity {
            return Err(StorageError::IdentityMismatch);
        }
        if self.registry.generations.len() >= MAX_GENERATIONS {
            return Err(StorageError::InputLimit);
        }
        let (g, writes) = self.make_generation(root, pid, Some(parent), &capture)?;
        let id = g.id;
        let mut next = self.copy_registry()?;
        next.profiles
            .iter_mut()
            .find(|p| p.id == pid)
            .ok_or(StorageError::Missing)?
            .latest = id;
        next.generations.push(g);
        self.commit(root, next, writes, vec![])?;
        Ok(GenerationId::from_bytes(id)?)
    }
    pub fn latest(&self, profile: ProfileId) -> Result<GenerationId, StorageError> {
        self.ready()?;
        let id = self
            .registry
            .profiles
            .iter()
            .find(|p| p.id == profile.to_bytes())
            .ok_or(StorageError::Missing)?
            .latest;
        Ok(GenerationId::from_bytes(id)?)
    }
    pub fn read_latest(
        &self,
        root: &RootKey,
        profile: ProfileId,
    ) -> Result<CredentialSet, StorageError> {
        let id = self.latest(profile)?.to_bytes();
        let g = self
            .registry
            .generations
            .iter()
            .find(|g| g.id == id)
            .ok_or(StorageError::Corrupt)?;
        self.read_generation(root, g)
    }
    pub fn remove(&mut self, root: &RootKey, profile: ProfileId) -> Result<(), StorageError> {
        self.ready()?;
        let id = profile.to_bytes();
        if !self.registry.active_known {
            return Err(StorageError::ActiveStateUnknown);
        }
        if self.registry.active == Some(id) {
            return Err(StorageError::ActiveProfile);
        }
        let mut next = self.copy_registry()?;
        if !next.profiles.iter().any(|p| p.id == id) {
            return Err(StorageError::Missing);
        }
        let ids: BTreeSet<_> = next
            .generations
            .iter()
            .filter(|g| g.profile == id)
            .map(|g| g.id)
            .collect();
        if next
            .holds
            .iter()
            .any(|h| h.generations.iter().any(|g| ids.contains(g)))
        {
            return Err(StorageError::Referenced);
        }
        let retire = next
            .blobs()
            .into_iter()
            .filter(|b| b.key.profile == id)
            .collect();
        next.profiles.retain(|p| p.id != id);
        next.generations.retain(|g| g.profile != id);
        self.commit(root, next, vec![], retire)
    }
    pub fn prune(&mut self, root: &RootKey) -> Result<(), StorageError> {
        self.ready()?;
        if !self.later_startup {
            return Err(StorageError::LaterStartupRequired);
        }
        self.verify_registry(root, &self.registry)?;
        self.inventory()?;
        let mut next = self.copy_registry()?;
        let keep: BTreeSet<_> = next
            .profiles
            .iter()
            .map(|p| p.latest)
            .chain(
                next.holds
                    .iter()
                    .flat_map(|h| h.generations.iter().copied()),
            )
            .collect();
        let retire = next
            .blobs()
            .into_iter()
            .filter(|b| !keep.contains(&b.key.generation))
            .collect();
        next.generations.retain(|g| keep.contains(&g.id));
        // The later startup was established before changing any selection.
        self.commit(root, next, vec![], retire)?;
        let mut deleting = self.state.clone();
        deleting.deleting = true;
        self.change_state(root, deleting)?;
        self.cleanup(root)
    }
    fn cleanup(&mut self, root: &RootKey) -> Result<(), StorageError> {
        self.fresh()?;
        self.verify_registry(root, &self.registry)?;
        let reachable: BTreeSet<_> = self
            .registry
            .blobs()
            .iter()
            .map(|b| b.key)
            .chain(self.state.current.iter().map(|b| b.key))
            .collect();
        if self
            .state
            .garbage
            .iter()
            .any(|b| reachable.contains(&b.key))
        {
            return Err(StorageError::Corrupt);
        }
        self.blocked = true;
        for b in &self.state.garbage {
            for name in [b.key.name(), format!("{}.stage", b.key.name())] {
                if let Some(bytes) = self.disk.read(&name)? {
                    b.check(&bytes)?;
                    self.disk.erase(&name, &bytes)?;
                }
            }
        }
        self.blocked = false;
        let mut clean = self.state.clone();
        clean.garbage.clear();
        clean.deleting = false;
        self.change_state(root, clean)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
