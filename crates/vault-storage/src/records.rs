#![forbid(unsafe_code)]
use crate::{ProfileText, StorageError};
use codex_accounts_vault::Identity;
use codex_accounts_vault_crypto::{EnvelopeContext, Purpose, RootKey};
use sha2::{Digest, Sha256};

pub(crate) type Id = [u8; 16];
pub(crate) const MAX_PROFILES: usize = 50;
pub(crate) const MAX_GENERATIONS: usize = 128; // whole-store conservative bound
pub(crate) const MAX_BLOBS: usize = 2304;
pub(crate) const MAX_METADATA: usize = 512 * 1024;
pub(crate) const MAX_FILE: usize = 4 * 1024 * 1024 + 66;

pub(crate) fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
pub(crate) fn random_id() -> Result<Id, StorageError> {
    let mut id = [0; 16];
    getrandom::fill(&mut id).map_err(|_| StorageError::KeyUnavailable)?;
    if id == [0; 16] {
        return Err(StorageError::KeyUnavailable);
    }
    id[6] = (id[6] & 0x0f) | 0x40; // UUID version 4, RFC 9562
    id[8] = (id[8] & 0x3f) | 0x80;
    Ok(id)
}
pub(crate) fn now() -> Result<u64, StorageError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| StorageError::InvalidData)
}

/// Only these generated keys reach filesystem code. No deserialized path strings.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Key {
    pub kind: u8,
    pub profile: Id,
    pub generation: Id,
    pub slot: u8,
}
impl Key {
    pub fn registry(root: &RootKey, id: Id) -> Self {
        Self {
            kind: 1,
            profile: root.identifier(),
            generation: id,
            slot: 0,
        }
    }
    pub fn generation(profile: Id, id: Id) -> Self {
        Self {
            kind: 2,
            profile,
            generation: id,
            slot: 0,
        }
    }
    pub fn resource(profile: Id, id: Id, slot: u8) -> Self {
        Self {
            kind: 3,
            profile,
            generation: id,
            slot,
        }
    }
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.profile == [0; 16]
            || self.generation == [0; 16]
            || !(1..=3).contains(&self.kind)
            || self.slot >= 16
            || (self.kind != 3 && self.slot != 0)
        {
            return Err(StorageError::Corrupt);
        }
        Ok(())
    }
    pub fn context(&self) -> Result<EnvelopeContext, StorageError> {
        self.validate()?;
        let purpose = match self.kind {
            1 => Purpose::Registry,
            2 => Purpose::Generation,
            _ => Purpose::CredentialResource,
        };
        Ok(EnvelopeContext::new(
            purpose,
            1,
            self.profile,
            self.generation,
            self.slot as u16,
        )?)
    }
    pub fn name(&self) -> String {
        let hex = |id: &Id| id.iter().map(|b| format!("{b:02x}")).collect::<String>();
        format!(
            "{}-{}-{}-{:02}.bin",
            hex(&self.profile),
            hex(&self.generation),
            self.kind,
            self.slot
        )
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Blob {
    pub key: Key,
    pub hash: [u8; 32],
    pub size: u32,
}
impl Blob {
    pub fn new(key: Key, data: &[u8]) -> Result<Self, StorageError> {
        key.validate()?;
        if data.len() > MAX_FILE {
            return Err(StorageError::InputLimit);
        }
        Ok(Self {
            key,
            hash: digest(data),
            size: data.len() as u32,
        })
    }
    pub fn check(&self, data: &[u8]) -> Result<(), StorageError> {
        if self.size as usize != data.len() || digest(data) != self.hash {
            Err(StorageError::ExternalChange)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Rule {
    pub slot: u8,
    pub json: bool,
    pub required: bool,
    pub blob: Option<Blob>,
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Generation {
    pub profile: Id,
    pub id: Id,
    pub parent: Option<Id>,
    pub schema: u32,
    pub captured: u64,
    pub rules: Vec<Rule>,
    pub manifest: Blob,
}
pub(crate) struct Profile {
    pub id: Id,
    pub label: ProfileText,
    pub domain: ProfileText,
    pub identity: Identity,
    pub latest: Id,
    pub created: u64,
}
#[derive(Clone)]
pub(crate) struct Hold {
    pub id: Id,
    pub generations: Vec<Id>,
}
pub(crate) struct Registry {
    pub revision: u64,
    pub profiles: Vec<Profile>,
    pub generations: Vec<Generation>,
    pub holds: Vec<Hold>,
    pub active_known: bool,
    pub active: Option<Id>,
}
impl Registry {
    pub fn empty() -> Self {
        Self {
            revision: 0,
            profiles: vec![],
            generations: vec![],
            holds: vec![],
            active_known: false,
            active: None,
        }
    }
    pub fn blobs(&self) -> Vec<Blob> {
        self.generations
            .iter()
            .flat_map(|g| {
                std::iter::once(g.manifest.clone())
                    .chain(g.rules.iter().filter_map(|r| r.blob.clone()))
            })
            .collect()
    }
    pub fn validate(&self) -> Result<(), StorageError> {
        use std::collections::BTreeSet;
        if self.profiles.len() > MAX_PROFILES
            || self.generations.len() > MAX_GENERATIONS
            || self.holds.len() > MAX_GENERATIONS
        {
            return Err(StorageError::InputLimit);
        }
        let pids: BTreeSet<_> = self.profiles.iter().map(|p| p.id).collect();
        let gids: BTreeSet<_> = self.generations.iter().map(|g| g.id).collect();
        let hids: BTreeSet<_> = self.holds.iter().map(|h| h.id).collect();
        if pids.len() != self.profiles.len()
            || gids.len() != self.generations.len()
            || hids.len() != self.holds.len()
            || pids.contains(&[0; 16])
            || gids.contains(&[0; 16])
            || hids.contains(&[0; 16])
        {
            return Err(StorageError::Corrupt);
        }
        if (!self.active_known && self.active.is_some())
            || self.active.is_some_and(|id| !pids.contains(&id))
        {
            return Err(StorageError::Corrupt);
        }
        for (i, p) in self.profiles.iter().enumerate() {
            if self.profiles[..i].iter().any(|q| p.identity == q.identity)
                || !self
                    .generations
                    .iter()
                    .any(|g| g.id == p.latest && g.profile == p.id)
            {
                return Err(StorageError::Corrupt);
            }
        }
        for g in &self.generations {
            if !pids.contains(&g.profile)
                || g.parent == Some(g.id)
                || g.schema == 0
                || g.rules.is_empty()
                || g.rules.len() > 16
                || g.manifest.size < 66
                || g.manifest.size as usize > MAX_METADATA + 66
                || g.manifest.key != Key::generation(g.profile, g.id)
            {
                return Err(StorageError::Corrupt);
            }
            // Retained parents must belong to this profile and precede the child.
            // Pruned ancestors may be absent, but a retained cycle is never valid.
            let mut seen = BTreeSet::new();
            let mut ancestor = g.parent;
            while let Some(id) = ancestor {
                if id == [0; 16] || id == g.id || !seen.insert(id) {
                    return Err(StorageError::Corrupt);
                }
                let Some(parent) = self.generations.iter().find(|p| p.id == id) else {
                    break;
                };
                if parent.profile != g.profile || parent.captured > g.captured {
                    return Err(StorageError::Corrupt);
                }
                ancestor = parent.parent;
            }
            let slots: BTreeSet<_> = g.rules.iter().map(|r| r.slot).collect();
            if slots.len() != g.rules.len() {
                return Err(StorageError::Corrupt);
            }
            let mut total = 0;
            for r in &g.rules {
                if r.slot >= 16 || (r.required && r.blob.is_none()) {
                    return Err(StorageError::Corrupt);
                }
                if let Some(b) = &r.blob {
                    if b.key != Key::resource(g.profile, g.id, r.slot)
                        || b.size < 66
                        || b.size as usize > 1024 * 1024 + 66
                    {
                        return Err(StorageError::Corrupt);
                    }
                    total += b.size as usize - 66;
                }
            }
            if total > 4 * 1024 * 1024 {
                return Err(StorageError::InputLimit);
            }
        }
        for h in &self.holds {
            let refs: BTreeSet<_> = h.generations.iter().copied().collect();
            if refs.is_empty() || refs.len() != h.generations.len() || !refs.is_subset(&gids) {
                return Err(StorageError::Corrupt);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    Clean,
    ControlRepairRequired,
    CommitPending,
    CleanupPending,
    Blocked,
}
/// An encrypted control record. `parent` binds a staged replacement to the exact
/// previous ciphertext. Only its root-derived fixed context is used to open it.
#[derive(Clone)]
pub(crate) struct State {
    pub sequence: u64,
    pub parent: Option<[u8; 32]>,
    pub current: Option<Blob>,
    pub next: Option<Blob>,
    pub writes: Vec<Blob>,
    pub inline: Vec<(Blob, Vec<u8>)>,
    pub garbage: Vec<Blob>,
    pub deleting: bool,
}
impl State {
    pub fn initial() -> Self {
        Self {
            sequence: 0,
            parent: None,
            current: None,
            next: None,
            writes: vec![],
            inline: vec![],
            garbage: vec![],
            deleting: false,
        }
    }
    pub fn validate(&self) -> Result<(), StorageError> {
        use std::collections::BTreeSet;
        if self.writes.len() > MAX_BLOBS || self.garbage.len() > MAX_BLOBS || self.inline.len() > 2
        {
            return Err(StorageError::InputLimit);
        }
        let keys: BTreeSet<_> = self.writes.iter().map(|b| b.key).collect();
        let garbage: BTreeSet<_> = self.garbage.iter().map(|b| b.key).collect();
        if keys.len() != self.writes.len()
            || garbage.len() != self.garbage.len()
            || !keys.is_disjoint(&garbage)
        {
            return Err(StorageError::Corrupt);
        }
        if self.next.is_none() && (!self.writes.is_empty() || !self.inline.is_empty()) {
            return Err(StorageError::Corrupt);
        }
        if self
            .next
            .as_ref()
            .is_some_and(|b| b.key.kind != 1 || !self.writes.contains(b))
            || self.current.as_ref().is_some_and(|b| b.key.kind != 1)
            || (self.deleting && self.next.is_some())
        {
            return Err(StorageError::Corrupt);
        }
        let inline_keys: BTreeSet<_> = self.inline.iter().map(|(b, _)| b.key).collect();
        if inline_keys.len() != self.inline.len() {
            return Err(StorageError::Corrupt);
        }
        for (b, data) in &self.inline {
            if b.key.kind == 3 || !self.writes.contains(b) || data.len() > MAX_METADATA + 66 {
                return Err(StorageError::Corrupt);
            }
            b.check(data)?;
        }
        for b in self
            .current
            .iter()
            .chain(self.next.iter())
            .chain(&self.writes)
            .chain(&self.garbage)
        {
            b.key.validate()?;
            if b.size < 66 || b.size as usize > MAX_FILE {
                return Err(StorageError::Corrupt);
            }
        }
        Ok(())
    }
}
