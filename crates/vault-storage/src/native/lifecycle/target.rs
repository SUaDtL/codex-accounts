//! CA-04C synthetic target resources. No production constructor or Effects impl.
//! This is NOT a qualified Desktop filesystem adapter. Retaining an old handle
//! denies in-place writers but delete sharing is necessary for POSIX replacement:
//! this primitive does not provide an atomic namespace compare-and-swap.
use super::super::*;
use super::home::HomeLock;
use codex_accounts_vault::{CredentialSet, Resource, ResourceId};
use zeroize::Zeroizing;

const LIMIT: usize = 1024 * 1024;
const STAGE_PREFIX: &str = "ca04c-synthetic-";
type Plain = Zeroizing<Vec<u8>>;
type Pinned = (File, Plain, Stamp);

pub(super) struct Target {
    path: PathBuf,
    lock: HomeLock,
    security: Security,
    slots: u16,
    #[cfg(test)]
    pub(super) boundary: usize,
    #[cfg(test)]
    pub(super) fail_at: Option<usize>,
    #[cfg(test)]
    pub(super) crash: bool,
}
impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SyntheticTarget([REDACTED])")
    }
}
impl Target {
    /// Only tests can construct this adapter, for a fresh protected synthetic
    /// directory. No consumer input, runtime flag or catalog record can enable it.
    #[cfg(test)]
    pub(super) fn open(path: &Path, slots: u16) -> Result<Self, StorageError> {
        if slots == 0 {
            return Err(StorageError::InvalidData);
        }
        let target = Self {
            path: path.to_owned(),
            lock: HomeLock::for_synthetic_target(path)?,
            security: Security::new()?,
            slots,
            boundary: 0,
            fail_at: None,
            crash: false,
        };
        target.validate()?;
        Ok(target)
    }
    pub(super) fn validate(&self) -> Result<(), StorageError> {
        self.lock.revalidate()?;
        let directory = open_file(
            &self.path,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ,
            OPEN_EXISTING,
            None,
        )?;
        check_object(&directory, true)?;
        // This exact DACL is a SYNTHETIC fixture contract, not an assertion about
        // the correct ACL of a real Desktop home. Existing permissions never change.
        self.security.check(&directory, true)
    }
    fn point(&mut self) -> Result<(), StorageError> {
        #[cfg(test)]
        {
            self.boundary += 1;
            if self.fail_at == Some(self.boundary) {
                if self.crash {
                    // Test child only. Deliberately no Drop/unwind cleanup.
                    super::target_tests::crash_at_native_boundary();
                }
                return Err(StorageError::Io);
            }
        }
        Ok(())
    }
    fn slot(&self, slot: u8) -> Result<(), StorageError> {
        if slot >= 16 || self.slots & (1u16 << slot) == 0 {
            return Err(StorageError::InvalidData);
        }
        Ok(())
    }
    fn live(&self, slot: u8) -> Result<PathBuf, StorageError> {
        self.slot(slot)?;
        Ok(self.path.join(format!("synthetic-resource-{slot:02}.bin")))
    }
    fn staged(&self, operation: Id, slot: u8) -> Result<PathBuf, StorageError> {
        self.slot(slot)?;
        if operation == [0; 16] {
            return Err(StorageError::InvalidData);
        }
        let id: String = operation.iter().map(|b| format!("{b:02x}")).collect();
        Ok(self
            .path
            .join(format!("{STAGE_PREFIX}{id}-{slot:02}.stage")))
    }
    fn read_plain(&self, file: &mut File) -> Result<Plain, StorageError> {
        let before = check_object(file, false)?;
        self.security.check(file, true)?;
        if before.size > LIMIT as u64 {
            return Err(StorageError::InputLimit);
        }
        let mut bytes = Zeroizing::new(vec![0; before.size as usize]);
        file.seek(SeekFrom::Start(0))
            .map_err(|_| StorageError::Io)?;
        file.read_exact(&mut bytes).map_err(|_| StorageError::Io)?;
        if stamp(file)? != before {
            return Err(StorageError::ExternalChange);
        }
        self.security.check(file, true)?;
        Ok(bytes)
    }
    fn pin(&self, path: &Path, access: u32, share: u32) -> Result<Option<Pinned>, StorageError> {
        self.validate()?;
        let mut file = match open_file(path, access | READ_CONTROL, share, OPEN_EXISTING, None) {
            Ok(file) => file,
            Err(StorageError::Missing) => {
                self.validate()?;
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let bytes = self.read_plain(&mut file)?;
        let identity = stamp(&file)?;
        self.validate()?;
        Ok(Some((file, bytes, identity)))
    }
    pub(super) fn read(&self, slot: u8) -> Result<Option<Plain>, StorageError> {
        Ok(self
            .pin(&self.live(slot)?, GENERIC_READ, FILE_SHARE_READ)?
            .map(|(_, bytes, _)| bytes))
    }
    pub(super) fn snapshot(&self) -> Result<Vec<Resource>, StorageError> {
        self.validate()?;
        let mut resources = Vec::new();
        let mut total = 0usize;
        for slot in 0..16 {
            if self.slots & (1 << slot) == 0 {
                continue;
            }
            let id = ResourceId::new(slot)?;
            resources.push(match self.read(slot)? {
                Some(bytes) => {
                    total += bytes.len();
                    if total > 4 * LIMIT {
                        return Err(StorageError::InputLimit);
                    }
                    Resource::present(id, bytes.to_vec())?
                }
                None => Resource::absent(id),
            });
        }
        self.validate()?;
        Ok(resources)
    }
    /// The coordinator has persisted staging_intent or restore_intent before
    /// this call. Absence needs no plaintext marker or invented companion file.
    pub(super) fn stage(
        &mut self,
        operation: Id,
        slot: u8,
        value: Option<&[u8]>,
    ) -> Result<(), StorageError> {
        let path = self.staged(operation, slot)?;
        self.inventory(operation, self.slots)?;
        let Some(bytes) = value else {
            return Ok(());
        };
        if bytes.len() > LIMIT {
            return Err(StorageError::InputLimit);
        }
        if let Some((_, existing, _)) = self.pin(&path, GENERIC_READ, 0)? {
            return if existing.as_slice() == bytes {
                Ok(())
            } else {
                Err(StorageError::ExternalChange)
            };
        }
        self.point()?;
        let mut file = open_file(
            &path,
            GENERIC_READ | GENERIC_WRITE | DELETE | READ_CONTROL,
            0,
            CREATE_NEW,
            Some(&self.security),
        )?;
        check_object(&file, false)?;
        self.security.check(&file, true)?;
        self.point()?;
        file.write_all(bytes).map_err(|_| StorageError::Io)?;
        self.point()?;
        file.sync_all().map_err(|_| StorageError::Io)?;
        self.point()?;
        if self.read_plain(&mut file)?.as_slice() != bytes {
            return Err(StorageError::ExternalChange);
        }
        self.validate()
    }
    /// Handle-bound content comparison, no truncate/copy/delete-then-rename fallback.
    /// This still REQUIRES externally established namespace-writer quiescence.
    pub(super) fn replace(
        &mut self,
        operation: Id,
        slot: u8,
        expected: Option<&[u8]>,
        value: Option<&[u8]>,
    ) -> Result<(), StorageError> {
        let live = self.live(slot)?;
        self.inventory(operation, self.slots)?;
        if expected.is_some_and(|b| b.len() > LIMIT) || value.is_some_and(|b| b.len() > LIMIT) {
            return Err(StorageError::InputLimit);
        }
        let deletion = value.is_none();
        let old = self.pin(
            &live,
            GENERIC_READ | if deletion { GENERIC_WRITE | DELETE } else { 0 },
            if deletion {
                0
            } else {
                FILE_SHARE_READ | FILE_SHARE_DELETE
            },
        )?;
        if old.as_ref().map(|(_, b, _)| b.as_slice()) != expected {
            return Err(StorageError::ExternalChange);
        }
        if expected == value {
            return Ok(());
        }
        match value {
            None => {
                let (file, _, _) = old.ok_or(StorageError::ExternalChange)?;
                self.point()?;
                self.dispose(&file)?;
                self.point()?;
                drop(file);
                self.point()?;
                if self.read(slot)?.is_some() {
                    return Err(StorageError::ExternalChange);
                }
            }
            Some(bytes) => {
                self.stage(operation, slot, Some(bytes))?;
                let (mut source, staged, identity) = self
                    .pin(
                        &self.staged(operation, slot)?,
                        GENERIC_READ | GENERIC_WRITE | DELETE,
                        0,
                    )?
                    .ok_or(StorageError::Missing)?;
                if staged.as_slice() != bytes {
                    return Err(StorageError::ExternalChange);
                }
                // Reopen the name while retaining the old handle. This refuses
                // replacement BEFORE this observation, not an atomic namespace CAS.
                let current = self.pin(&live, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_DELETE)?;
                if current.as_ref().map(|(_, b, _)| b.as_slice()) != expected
                    || current.as_ref().map(|(_, _, s)| (s.volume, s.id))
                        != old.as_ref().map(|(_, _, s)| (s.volume, s.id))
                {
                    return Err(StorageError::ExternalChange);
                }
                self.point()?;
                let name: Vec<u16> = live.as_os_str().encode_wide().collect();
                let size = std::mem::offset_of!(FILE_RENAME_INFO, FileName) + name.len() * 2;
                let mut storage = vec![0u64; size.div_ceil(8)];
                let info = storage.as_mut_ptr().cast::<FILE_RENAME_INFO>();
                // SAFETY: aligned zeroed SDK structure, bounded same-directory
                // destination under retained ancestors, no caller-authored path.
                unsafe {
                    // SDK FILE_RENAME_REPLACE_IF_EXISTS | FILE_RENAME_POSIX_SEMANTICS.
                    // Zero for an absent target gives atomic no-clobber creation.
                    (*info).Anonymous.Flags = if expected.is_some() { 0x1 | 0x2 } else { 0 };
                    (*info).RootDirectory = null_mut();
                    (*info).FileNameLength = (name.len() * 2) as u32;
                    std::ptr::copy_nonoverlapping(
                        name.as_ptr(),
                        (*info).FileName.as_mut_ptr(),
                        name.len(),
                    );
                    if SetFileInformationByHandle(
                        source.as_raw_handle(),
                        FileRenameInfoEx,
                        info.cast(),
                        size as u32,
                    ) == 0
                    {
                        return Err(match last() {
                            StorageError::AlreadyExists => StorageError::ExternalChange,
                            error => error,
                        });
                    }
                }
                self.point()?;
                source.sync_all().map_err(|_| StorageError::Io)?;
                self.point()?;
                let after = check_object(&source, false)?;
                if (after.volume, after.id) != (identity.volume, identity.id)
                    || self.read_plain(&mut source)?.as_slice() != bytes
                {
                    return Err(StorageError::ExternalChange);
                }
                drop(source);
                let (_, actual, after) = self
                    .pin(&live, GENERIC_READ, FILE_SHARE_READ)?
                    .ok_or(StorageError::ExternalChange)?;
                if actual.as_slice() != bytes
                    || (after.volume, after.id) != (identity.volume, identity.id)
                {
                    return Err(StorageError::ExternalChange);
                }
            }
        }
        self.validate()
    }
    fn dispose(&self, file: &File) -> Result<(), StorageError> {
        check_object(file, false)?;
        self.security.check(file, true)?;
        let info = FILE_DISPOSITION_INFO { DeleteFile: true };
        // SAFETY: pinned validated DELETE handle, exact initialized SDK value.
        if unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle(),
                FileDispositionInfo,
                (&info as *const FILE_DISPOSITION_INFO).cast(),
                std::mem::size_of_val(&info) as u32,
            )
        } == 0
        {
            return Err(last());
        }
        file.sync_all().map_err(|_| StorageError::Io)
    }
    /// Inventory is bounded and exact; a parseable random name is NOT registration.
    fn inventory(
        &self,
        operation: Id,
        registered: u16,
    ) -> Result<Vec<(u8, PathBuf)>, StorageError> {
        self.validate()?;
        if registered & !self.slots != 0 {
            return Err(StorageError::InvalidData);
        }
        let mut stages = Vec::new();
        let mut count = 0;
        for entry in std::fs::read_dir(&self.path).map_err(|_| StorageError::Io)? {
            count += 1;
            if count > 32 {
                return Err(StorageError::InputLimit);
            }
            let entry = entry.map_err(|_| StorageError::Io)?;
            let path = entry.path();
            let mut known = false;
            for slot in 0..16 {
                if self.slots & (1 << slot) == 0 {
                    continue;
                }
                if path == self.live(slot)? {
                    known = true;
                    break;
                }
                if path == self.staged(operation, slot)? {
                    if registered & (1 << slot) == 0 {
                        return Err(StorageError::ExternalChange);
                    }
                    stages.push((slot, path.clone()));
                    known = true;
                    break;
                }
            }
            if !known {
                return Err(StorageError::ExternalChange);
            }
        }
        self.validate()?;
        Ok(stages)
    }
    /// Allowed bytes come from authenticated source/current-target/original-target
    /// generations, not from a stage filename or an untrusted on-disk header.
    pub(super) fn cleanup(
        &mut self,
        operation: Id,
        registered: u16,
        allowed: &[CredentialSet],
    ) -> Result<(), StorageError> {
        let stages = self.inventory(operation, registered)?;
        // Validate ALL candidates before deleting any. Each delete rechecks its
        // pinned bytes, shape and ACL; partial cleanup is safe to retry on restart.
        for (slot, path) in &stages {
            self.cleanup_candidate(*slot, path, allowed)?;
        }
        for (slot, path) in stages {
            let Some(file) = self.cleanup_candidate(slot, &path, allowed)? else {
                continue;
            };
            self.point()?;
            self.dispose(&file)?;
            drop(file);
            self.point()?;
            if self.pin(&path, GENERIC_READ, 0)?.is_some() {
                return Err(StorageError::ExternalChange);
            }
        }
        self.validate()
    }
    fn cleanup_candidate(
        &self,
        slot: u8,
        path: &Path,
        allowed: &[CredentialSet],
    ) -> Result<Option<File>, StorageError> {
        let Some((file, bytes, _)) = self.pin(path, GENERIC_READ | GENERIC_WRITE | DELETE, 0)?
        else {
            return Ok(None);
        };
        let id = ResourceId::new(slot)?;
        if !allowed
            .iter()
            .any(|set| set.resource(id).and_then(Resource::as_bytes) == Some(bytes.as_slice()))
        {
            return Err(StorageError::ExternalChange);
        }
        Ok(Some(file))
    }
}
