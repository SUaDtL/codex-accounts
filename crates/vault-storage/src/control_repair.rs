#![forbid(unsafe_code)]
//! Bounded storage-only recovery. Evidence is never selected as registry state,
//! never replayed into a credential slot, and never pruned automatically.
use super::*;
use zeroize::Zeroizing;

const MAX_EVIDENCE_FILES: usize = 32;
const MAX_EVIDENCE_BYTES: usize = 2 * 1024 * 1024;

pub(super) fn evidence_name(name: &str) -> bool {
    let name = name.strip_suffix(".stage").unwrap_or(name);
    let Some(id) = name
        .strip_prefix("recovery-")
        .and_then(|s| s.strip_suffix(".bin"))
    else {
        return false;
    };
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn check_evidence(disk: &impl Files, names: &[String]) -> Result<(), StorageError> {
    let records: Vec<_> = names.iter().filter(|n| evidence_name(n)).collect();
    if records.len() > MAX_EVIDENCE_FILES {
        return Err(StorageError::InputLimit);
    }
    for name in records {
        let bytes = disk.read(name)?.ok_or(StorageError::ExternalChange)?;
        if bytes.len() > MAX_EVIDENCE_BYTES {
            return Err(StorageError::InputLimit);
        }
    }
    Ok(())
}
impl<D: Files> Storage<D> {
    pub(super) fn interrupted_control(
        disk: D,
        root: &RootKey,
        current: Option<Vec<u8>>,
        stage: Option<Vec<u8>>,
    ) -> Result<Self, StorageError> {
        if stage.as_ref().is_some_and(|b| b.len() > MAX_METADATA + 66) {
            return Err(StorageError::InputLimit);
        }
        // Authenticate the committed state independently. A corrupt committed
        // control or key cannot become an empty/new vault through this operation.
        let state = current
            .as_deref()
            .map(|b| open_state(root, b))
            .transpose()?
            .unwrap_or_else(State::initial);
        let registry = match &state.current {
            Some(b) => Self::load_registry(&disk, root, b)?,
            None if current.is_none() || state.next.is_some() => Registry::empty(),
            None => return Err(StorageError::Corrupt),
        };
        let store = Self {
            disk,
            state,
            state_bytes: current,
            registry,
            later_startup: false,
            blocked: false,
            control_repair: true,
            torn_control: stage.map(Zeroizing::new),
        };
        store.verify_registry(root, &store.registry)?;
        store.inventory()?;
        Ok(store)
    }
    fn check_control_snapshot(&self) -> Result<(), StorageError> {
        if self.disk.read("state.bin")? != self.state_bytes {
            return Err(StorageError::ExternalChange);
        }
        Ok(())
    }
    /// Preserve first, compare again, then remove only this unpublished file.
    /// An interrupted evidence write retains BOTH the original and its partial
    /// encrypted copy. Retry uses a fresh random name/nonce, never overwrites it.
    pub(super) fn archive_unpublished(
        &mut self,
        root: &RootKey,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), StorageError> {
        if !name.ends_with(".stage") || name.len() > 110 || bytes.len() > 1024 * 1024 + 66 {
            return Err(StorageError::InputLimit);
        }
        self.check_control_snapshot()?;
        self.inventory()?;
        if self.disk.read(name)?.as_deref() != Some(bytes) {
            return Err(StorageError::ExternalChange);
        }
        let names = self.disk.list()?;
        if names.iter().filter(|n| evidence_name(n)).count() >= MAX_EVIDENCE_FILES {
            return Err(StorageError::InputLimit);
        }
        let id = random_id()?;
        let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
        let destination = format!("recovery-{hex}.bin");
        let context = EnvelopeContext::new(Purpose::Journal, 1, root.identifier(), id, 0)?;
        let mut record = Zeroizing::new(Vec::new());
        record.extend_from_slice(b"CAREC001");
        record.push(u8::from(self.state_bytes.is_some()));
        record.extend_from_slice(&self.state_bytes.as_deref().map(digest).unwrap_or([0; 32]));
        record.extend_from_slice(&(name.len() as u16).to_le_bytes());
        record.extend_from_slice(name.as_bytes());
        record.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        record.extend_from_slice(bytes);
        let sealed = root.seal(&context, &record)?;
        self.disk.stage(&destination, sealed.as_bytes())?;
        self.disk.publish(&destination, None, sealed.as_bytes())?;
        let saved = self
            .disk
            .read(&destination)?
            .ok_or(StorageError::RecoveryRequired)?;
        if saved != sealed.as_bytes()
            || root
                .open(&context, &Envelope::from_bytes(&saved)?)?
                .expose()
                != record.as_slice()
        {
            return Err(StorageError::ExternalChange);
        }
        self.check_control_snapshot()?;
        // erase must independently compare through the pinned native handle.
        self.disk.erase(name, bytes)
    }
    pub fn recover_control(&mut self, root: &RootKey) -> Result<(), StorageError> {
        if !self.control_repair {
            return Err(StorageError::InvalidData);
        }
        self.verify_registry(root, &self.registry)?;
        self.check_control_snapshot()?;
        self.inventory()?;
        let expected = self.torn_control.as_deref().map(Vec::as_slice);
        let observed = self.disk.read("state.bin.stage")?;
        if observed.as_deref() != expected {
            return Err(StorageError::ExternalChange);
        }
        self.blocked = true;
        if let Some(bytes) = observed {
            let bytes = Zeroizing::new(bytes);
            self.archive_unpublished(root, "state.bin.stage", &bytes)?;
        }
        self.torn_control = None;
        self.control_repair = false;
        self.blocked = false;
        self.later_startup = false;
        // Interrupted first control publication can initialize ONLY an empty
        // registry using the SAME surviving key, with no unexpected data files.
        if self.state_bytes.is_none() {
            self.commit(root, Registry::empty(), vec![], vec![])?;
        }
        Ok(())
    }
}
