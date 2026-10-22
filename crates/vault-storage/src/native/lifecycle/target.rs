//! Fixed file-backend target and separate synthetic test layout. Retaining an old handle
//! denies in-place writers but delete sharing is necessary for POSIX replacement:
//! this primitive does not provide an atomic namespace compare-and-swap.
use super::super::*;
use super::home::HomeLock;
use codex_accounts_vault::{CredentialSet, Resource, ResourceId};
use zeroize::Zeroizing;

const LIMIT: usize = 1024 * 1024;
const STAGE_PREFIX: &str = "ca04c-synthetic-";
const FILE_STAGE_PREFIX: &str = ".codex-accounts-";
type Plain = Zeroizing<Vec<u8>>;
type Pinned = (File, Plain, Stamp);

pub(super) struct Target {
    path: PathBuf,
    lock: HomeLock,
    security: Security,
    slots: u16,
    file_backend: bool,
    #[cfg(test)]
    pub(super) boundary: usize,
    #[cfg(test)]
    pub(super) fail_at: Option<usize>,
    #[cfg(test)]
    pub(super) crash: bool,
}

impl std::fmt::Debug for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.file_backend {
            "FileBackendTarget([REDACTED])"
        } else {
            "SyntheticTarget([REDACTED])"
        })
    }
}
impl Target {
    /// The caller has selected a file credential backend and must separately
    /// quiesce other writers before mutation. No path or ACL is repaired here.
    pub(super) fn open_file_backend(path: &Path) -> Result<Self, StorageError> {
        let target = Self {
            path: path.to_owned(),
            lock: HomeLock::acquire_for_transaction(path)?,
            security: Security::new()?,
            slots: 1,
            file_backend: true,
            #[cfg(test)]
            boundary: 0,
            #[cfg(test)]
            fail_at: None,
            #[cfg(test)]
            crash: false,
        };
        target.validate()?;
        Ok(target)
    }
    /// Short shared read for usage observation. No helper, file write or account
    /// association update. Existing clients may replace auth.json during reads;
    /// reject a changed object rather than publishing mixed credentials.
    pub(super) fn read_usage_auth(&self) -> Result<Plain, StorageError> {
        let path = self.live(0)?;
        let share = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
        let mut file = open_file(
            &path,
            GENERIC_READ | READ_CONTROL,
            share,
            OPEN_EXISTING,
            None,
        )?;
        let bytes = self.read_plain(&mut file)?;
        let current = open_file(
            &path,
            GENERIC_READ | READ_CONTROL,
            share,
            OPEN_EXISTING,
            None,
        )?;
        if stamp(&file)? != stamp(&current)? {
            return Err(StorageError::ExternalChange);
        }
        self.lock.revalidate()?;
        Ok(bytes)
    }

    /// Only tests can select the synthetic multi-resource layout, for a fresh
    /// protected directory. Production always uses the fixed file-backend layout.
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
            file_backend: false,
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
        // HomeLock also requires the current user to own the leaf. Real homes
        // may have inherited administrator/SYSTEM access; fixture ACLs stay exact.
        self.security.check(&directory, !self.file_backend)?;
        // cap_sid is unrelated filesystem sandbox capability state, not account
        // authentication: openai/codex codex-rs/windows-sandbox-rs/src/cap.rs.
        // The fixed file backend owns only auth.json; never read or replace SIDs.
        Ok(())
    }
    fn check_resource(&self, file: &File) -> Result<(), StorageError> {
        if self.file_backend {
            self.security.check_credential(file)
        } else {
            self.security.check(file, true)
        }
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
        Ok(if self.file_backend {
            self.path.join("auth.json")
        } else {
            self.path.join(format!("synthetic-resource-{slot:02}.bin"))
        })
    }
    fn staged(&self, operation: Id, slot: u8) -> Result<PathBuf, StorageError> {
        self.slot(slot)?;
        if operation == [0; 16] {
            return Err(StorageError::InvalidData);
        }
        let id: String = operation.iter().map(|b| format!("{b:02x}")).collect();
        let prefix = if self.file_backend {
            FILE_STAGE_PREFIX
        } else {
            STAGE_PREFIX
        };
        Ok(self.path.join(format!("{prefix}{id}-{slot:02}.stage")))
    }
    fn read_plain(&self, file: &mut File) -> Result<Plain, StorageError> {
        let before = check_object(file, false)?;
        self.check_resource(file)?;
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
        self.check_resource(file)?;
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
        let mut file = open_temporary_file(
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
        self.check_resource(file)?;
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
            if count > if self.file_backend { 4096 } else { 32 } {
                return Err(StorageError::InputLimit);
            }
            let entry = entry.map_err(|_| StorageError::Io)?;
            let path = entry.path();
            // A real home contains configuration/history and other application
            // files. Only our fixed credential and stage namespace is relevant;
            // do not open, copy, or interpret unrelated entries.
            if self.file_backend
                && !entry
                    .file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .starts_with(FILE_STAGE_PREFIX)
            {
                continue;
            }
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
    /// Explicit recovery only. Normal cleanup never adopts unknown plaintext.
    /// All staged objects stay exclusively pinned until their ciphertext is
    /// committed to the existing journal and verified by the core callback.
    pub(super) fn preserve_then_remove(
        &mut self,
        operation: Id,
        registered: u16,
        preserve: &mut dyn FnMut(
            Vec<Resource>,
        ) -> Result<(), crate::engine::coordinator::SwitchError>,
    ) -> Result<(), crate::engine::coordinator::SwitchError> {
        use crate::{engine::coordinator::SwitchError, journal::Failure};
        let refusal = |error| {
            SwitchError::Refused(match error {
                StorageError::ExternalChange | StorageError::UnsafePath => Failure::ExternalChange,
                StorageError::InputLimit | StorageError::InvalidData => Failure::InvalidData,
                StorageError::Busy => Failure::Busy,
                _ => Failure::Cleanup,
            })
        };
        let stages = self.inventory(operation, registered).map_err(refusal)?;
        let mut held = Vec::new();
        let mut resources = Vec::new();
        let mut total = 0usize;
        for (slot, path) in stages {
            let (file, bytes, identity) = self
                .pin(&path, GENERIC_READ | GENERIC_WRITE | DELETE, 0)
                .map_err(refusal)?
                .ok_or(Failure::ExternalChange)?;
            total += bytes.len();
            if total > 4 * LIMIT {
                return Err(StorageError::InputLimit.into());
            }
            resources.push(
                Resource::present(
                    ResourceId::new(slot).map_err(StorageError::from)?,
                    bytes.to_vec(),
                )
                .map_err(StorageError::from)?,
            );
            held.push((file, bytes, identity));
        }
        if held.is_empty() {
            return Ok(());
        }
        self.point().map_err(refusal)?;
        preserve(resources)?;
        self.point().map_err(refusal)?;
        for (mut file, bytes, identity) in held {
            self.validate().map_err(refusal)?;
            if stamp(&file).map_err(refusal)? != identity
                || self.read_plain(&mut file).map_err(refusal)?.as_slice() != bytes.as_slice()
            {
                return Err(Failure::ExternalChange.into());
            }
            self.point().map_err(refusal)?;
            self.dispose(&file).map_err(refusal)?;
            self.point().map_err(refusal)?;
            drop(file);
            self.point().map_err(refusal)?;
        }
        if !self
            .inventory(operation, registered)
            .map_err(refusal)?
            .is_empty()
        {
            return Err(Failure::ExternalChange.into());
        }
        self.validate().map_err(refusal)?;
        Ok(())
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

#[cfg(test)]
mod file_backend_tests {
    use super::*;

    fn home() -> super::super::super::tests::Sandbox {
        let fixture = super::super::super::tests::Sandbox::new();
        let security = Security::new().unwrap();
        assert_ne!(
            unsafe { CreateDirectoryW(wide(&fixture.0).unwrap().as_ptr(), &security.attrs()) },
            0
        );
        fixture
    }

    #[test]
    fn file_backend_replaces_restores_and_leaves_unrelated_entries_alone() {
        let fixture = home();
        let sandbox_sids = b"{\"workspace\":\"SYNTHETIC_SID\",\"readonly\":\"SYNTHETIC_READONLY\"}";
        std::fs::write(fixture.0.join("cap_sid"), sandbox_sids).unwrap();
        for n in 0..40 {
            std::fs::write(
                fixture.0.join(format!("synthetic-unrelated-{n}")),
                b"UNCHANGED",
            )
            .unwrap();
        }
        let mut target = Target::open_file_backend(&fixture.0).unwrap();
        let a = b"{\"synthetic\":\"a\"}";
        let b = b"{\"synthetic\":\"b\"}";
        target.replace([1; 16], 0, None, Some(a)).unwrap();
        assert_eq!(target.live(0).unwrap(), fixture.0.join("auth.json"));
        assert_eq!(target.read(1).unwrap_err(), StorageError::InvalidData);
        target.replace([2; 16], 0, Some(a), Some(b)).unwrap();
        assert_eq!(target.read(0).unwrap().unwrap().as_slice(), b);
        assert_eq!(
            target.replace([3; 16], 0, Some(a), Some(a)),
            Err(StorageError::ExternalChange)
        );
        target.replace([4; 16], 0, Some(b), Some(a)).unwrap();
        assert_eq!(target.read(0).unwrap().unwrap().as_slice(), a);
        assert_eq!(
            std::fs::read(fixture.0.join("cap_sid")).unwrap(),
            sandbox_sids
        );
        for n in 0..40 {
            assert_eq!(
                std::fs::read(fixture.0.join(format!("synthetic-unrelated-{n}"))).unwrap(),
                b"UNCHANGED"
            );
        }
        std::fs::write(
            fixture.0.join(".codex-accounts-unknown.stage"),
            b"DO_NOT_ADOPT",
        )
        .unwrap();
        assert_eq!(
            target.stage([5; 16], 0, Some(b)),
            Err(StorageError::ExternalChange)
        );
        assert_eq!(target.read(0).unwrap().unwrap().as_slice(), a);
        assert!(target.snapshot().is_ok());
    }

    #[test]
    fn file_backend_accepts_ordinary_admin_acl_but_refuses_untrusted_reader() {
        let fixture = home();
        let mut target = Target::open_file_backend(&fixture.0).unwrap();
        let bytes = b"{\"synthetic\":\"not-a-credential\"}";
        target.replace([1; 16], 0, None, Some(bytes)).unwrap();
        let file = open_file(
            &fixture.0.join("auth.json"),
            WRITE_DAC | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            OPEN_EXISTING,
            None,
        )
        .unwrap();
        let add_grant = |sid: &[u8], mask| {
            let mut old_acl = null_mut();
            let mut descriptor = null_mut();
            assert_eq!(
                unsafe {
                    GetSecurityInfo(
                        file.as_raw_handle(),
                        SE_FILE_OBJECT,
                        DACL_SECURITY_INFORMATION,
                        null_mut(),
                        null_mut(),
                        &mut old_acl,
                        null_mut(),
                        &mut descriptor,
                    )
                },
                0
            );
            let _descriptor = Allocation(descriptor);
            let entry = EXPLICIT_ACCESS_W {
                grfAccessPermissions: mask,
                grfAccessMode: GRANT_ACCESS,
                grfInheritance: 0,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: null_mut(),
                    MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_UNKNOWN,
                    ptstrName: sid.as_ptr().cast_mut().cast(),
                },
            };
            let mut acl = null_mut();
            assert_eq!(unsafe { SetEntriesInAclW(1, &entry, old_acl, &mut acl) }, 0);
            let _acl = Allocation(acl.cast());
            // Only this fresh synthetic file is changed; real ACLs are never repaired.
            assert_eq!(
                unsafe {
                    SetSecurityInfo(
                        file.as_raw_handle(),
                        SE_FILE_OBJECT,
                        DACL_SECURITY_INFORMATION | UNPROTECTED_DACL_SECURITY_INFORMATION,
                        null_mut(),
                        null_mut(),
                        acl,
                        null_mut(),
                    )
                },
                0
            );
        };
        add_grant(&target.security.admins, FILE_ALL_ACCESS);
        target.security.check_credential(&file).unwrap();
        assert!(target.security.check(&file, true).is_err());
        add_grant(&known_sid(WinWorldSid).unwrap(), FILE_GENERIC_READ);
        assert_eq!(
            target.security.check_credential(&file),
            Err(StorageError::UnsafePath)
        );
        drop(file);
        assert_eq!(target.read(0).unwrap_err(), StorageError::UnsafePath);
    }
    #[test]
    #[ignore = "requires the official local CodexSandboxUsers group"]
    fn file_backend_accepts_codex_sandbox_read_only_but_refuses_write() {
        let fixture = home();
        let mut target = Target::open_file_backend(&fixture.0).unwrap();
        let bytes = b"{\"synthetic\":\"not-a-credential\"}";
        target.replace([1; 16], 0, None, Some(bytes)).unwrap();
        let file = open_file(
            &fixture.0.join("auth.json"),
            WRITE_DAC | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            OPEN_EXISTING,
            None,
        )
        .unwrap();
        let add_grant = |sid: &[u8], mask| {
            let mut old_acl = null_mut();
            let mut descriptor = null_mut();
            assert_eq!(
                unsafe {
                    GetSecurityInfo(
                        file.as_raw_handle(),
                        SE_FILE_OBJECT,
                        DACL_SECURITY_INFORMATION,
                        null_mut(),
                        null_mut(),
                        &mut old_acl,
                        null_mut(),
                        &mut descriptor,
                    )
                },
                0
            );
            let _descriptor = Allocation(descriptor);
            let entry = EXPLICIT_ACCESS_W {
                grfAccessPermissions: mask,
                grfAccessMode: GRANT_ACCESS,
                grfInheritance: 0,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: null_mut(),
                    MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_UNKNOWN,
                    ptstrName: sid.as_ptr().cast_mut().cast(),
                },
            };
            let mut acl = null_mut();
            assert_eq!(unsafe { SetEntriesInAclW(1, &entry, old_acl, &mut acl) }, 0);
            let _acl = Allocation(acl.cast());
            // Only this fresh synthetic file is changed; real ACLs are never repaired.
            assert_eq!(
                unsafe {
                    SetSecurityInfo(
                        file.as_raw_handle(),
                        SE_FILE_OBJECT,
                        DACL_SECURITY_INFORMATION | UNPROTECTED_DACL_SECURITY_INFORMATION,
                        null_mut(),
                        null_mut(),
                        acl,
                        null_mut(),
                    )
                },
                0
            );
        };
        let mut computer = [0_u16; 256];
        let mut computer_len = computer.len() as u32;
        assert_ne!(
            unsafe {
                windows_sys::Win32::System::WindowsProgramming::GetComputerNameW(
                    computer.as_mut_ptr(),
                    &mut computer_len,
                )
            },
            0
        );
        let mut name = computer[..computer_len as usize].to_vec();
        name.extend("\\CodexSandboxUsers\0".encode_utf16());
        let mut sid = [0_u64; 16];
        let mut sid_len = std::mem::size_of_val(&sid) as u32;
        let mut domain = [0_u16; 256];
        let mut domain_len = domain.len() as u32;
        let mut kind = 0;
        assert_ne!(
            unsafe {
                LookupAccountNameW(
                    null(),
                    name.as_ptr(),
                    sid.as_mut_ptr().cast(),
                    &mut sid_len,
                    domain.as_mut_ptr(),
                    &mut domain_len,
                    &mut kind,
                )
            },
            0
        );
        assert_eq!(kind, SidTypeAlias);
        let sid = sid_copy(sid.as_mut_ptr().cast()).unwrap();
        add_grant(&sid, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE);
        target.security.check_credential(&file).unwrap();
        assert!(target.security.check(&file, true).is_err());
        assert_eq!(target.read(0).unwrap().unwrap().as_slice(), bytes);
        add_grant(&sid, FILE_WRITE_DATA);
        assert_eq!(
            target.security.check_credential(&file),
            Err(StorageError::UnsafePath)
        );
        drop(file);
        assert_eq!(target.read(0).unwrap_err(), StorageError::UnsafePath);
    }
}
