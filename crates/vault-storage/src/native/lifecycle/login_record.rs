//! One immutable encrypted onboarding intent. Publication precedes scratch-home
//! creation; the intent remains authoritative until exact scratch cleanup ends.
use super::*;
use codex_accounts_vault_crypto::{Envelope, EnvelopeContext, Purpose};

const LIVE: &str = "onboarding-v1.bin";
const PENDING: &str = "onboarding-v1.pending";
const LIMIT: u64 = 256;
const MAGIC: &[u8; 8] = b"CALOGIN1";
#[cfg(test)]
#[path = "login_record_tests.rs"]
mod tests;

pub(super) fn is_record_name(name: &std::ffi::OsStr) -> bool {
    name == LIVE || name == PENDING
}
fn context(root: &RootKey) -> Result<EnvelopeContext, StorageError> {
    // The fixed generation is a dedicated domain, unrelated to random switch
    // IDs and the root/root main-control context. Plaintext has its own magic.
    Ok(EnvelopeContext::new(
        Purpose::Journal,
        0x4c4f474e,
        root.identifier(),
        *b"LoginOnboarding1",
        0,
    )?)
}
pub(super) struct Onboarding {
    operation: Id,
    file: File,
    stamp: Stamp,
}
impl Onboarding {
    pub(super) fn home(&self, namespace: &Namespace) -> PathBuf {
        let id: String = self
            .operation
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        namespace.path.join(format!("login-{id}"))
    }
    pub(super) fn begin(
        namespace: &Namespace,
        root: &RootKey,
        intended: Option<ProfileId>,
    ) -> Result<Self, StorageError> {
        namespace.lock.revalidate()?;
        if !namespace.children()?.is_empty()
            || Self::read(namespace, root)?.is_some()
            || has_unpublished(namespace)?
        {
            return Err(StorageError::TemporaryCleanupRequired);
        }
        let operation = random_id()?;
        let mut plaintext = Zeroizing::new(Vec::with_capacity(41));
        plaintext.extend_from_slice(MAGIC);
        plaintext.extend_from_slice(&operation);
        plaintext.push(u8::from(intended.is_some()));
        plaintext.extend_from_slice(&intended.map_or([0; 16], ProfileId::to_bytes));
        let encrypted = root.seal(&context(root)?, &plaintext)?;
        let mut file = open_file(
            &namespace.path.join(PENDING),
            GENERIC_READ | GENERIC_WRITE | READ_CONTROL | DELETE,
            0,
            CREATE_NEW,
            Some(&Security::new()?),
        )?;
        file.write_all(encrypted.as_bytes())
            .map_err(|_| StorageError::Io)?;
        file.sync_all().map_err(|_| StorageError::Io)?;
        if read_record_bytes(&mut file)?.as_slice() != encrypted.as_bytes() {
            return Err(StorageError::ExternalChange);
        }
        root.open(
            &context(root)?,
            &Envelope::from_bytes(encrypted.as_bytes())?,
        )?;
        publish(&file, &namespace.path.join(LIVE))?;
        file.sync_all().map_err(|_| StorageError::Io)?;
        let original = check_object(&file, false)?;
        drop(file);
        let loaded = Self::read(namespace, root)?.ok_or(StorageError::ExternalChange)?;
        if loaded.operation != operation
            || (loaded.stamp.volume, loaded.stamp.id) != (original.volume, original.id)
        {
            return Err(StorageError::ExternalChange);
        }
        Ok(loaded)
    }
    pub(super) fn read(
        namespace: &Namespace,
        root: &RootKey,
    ) -> Result<Option<Self>, StorageError> {
        namespace.lock.revalidate()?;
        let Some(mut file) = open_record(&namespace.path.join(LIVE))? else {
            return Ok(None);
        };
        let original = check_object(&file, false)?;
        let bytes = read_record_bytes(&mut file)?;
        let plaintext = root.open(&context(root)?, &Envelope::from_bytes(&bytes)?)?;
        let plain = plaintext.expose();
        if plain.len() != 41 || &plain[..8] != MAGIC || plain[24] > 1 {
            return Err(StorageError::Corrupt);
        }
        let operation: Id = plain[8..24].try_into().map_err(|_| StorageError::Corrupt)?;
        let profile: Id = plain[25..41]
            .try_into()
            .map_err(|_| StorageError::Corrupt)?;
        if operation == [0; 16] || (plain[24] == 0) != (profile == [0; 16]) {
            return Err(StorageError::Corrupt);
        }
        if stamp(&file)? != original {
            return Err(StorageError::ExternalChange);
        }
        Ok(Some(Self {
            operation,
            file,
            stamp: original,
        }))
    }
    pub(super) fn complete(&self, namespace: &Namespace) -> Result<(), StorageError> {
        namespace.lock.revalidate()?;
        if !namespace.children()?.is_empty() || has_unpublished(namespace)? {
            return Err(StorageError::TemporaryCleanupRequired);
        }
        if check_object(&self.file, false)? != self.stamp {
            return Err(StorageError::ExternalChange);
        }
        Security::new()?.check(&self.file, true)?;
        dispose(&self.file)
    }
}
fn open_record(path: &Path) -> Result<Option<File>, StorageError> {
    let file = match open_file(
        path,
        GENERIC_READ | READ_CONTROL | DELETE,
        FILE_SHARE_READ,
        OPEN_EXISTING,
        None,
    ) {
        Ok(file) => file,
        Err(StorageError::Missing) => return Ok(None),
        Err(error) => return Err(error),
    };
    if check_object(&file, false)?.size > LIMIT {
        return Err(StorageError::InputLimit);
    }
    Security::new()?.check(&file, true)?;
    Ok(Some(file))
}
fn read_record_bytes(file: &mut File) -> Result<Zeroizing<Vec<u8>>, StorageError> {
    let before = check_object(file, false)?;
    Security::new()?.check(file, true)?;
    if before.size > LIMIT {
        return Err(StorageError::InputLimit);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| StorageError::Io)?;
    let mut bytes = Zeroizing::new(vec![0; before.size as usize]);
    file.read_exact(&mut bytes).map_err(|_| StorageError::Io)?;
    if stamp(file)? != before {
        return Err(StorageError::ExternalChange);
    }
    Ok(bytes)
}
pub(super) fn has_unpublished(namespace: &Namespace) -> Result<bool, StorageError> {
    Ok(open_record(&namespace.path.join(PENDING))?.is_some())
}
pub(super) fn discard_unpublished(namespace: &Namespace) -> Result<(), StorageError> {
    namespace.lock.revalidate()?;
    // No home can have been created by an unpublished intent. A torn write is
    // therefore disposable only with an empty scratch inventory and no live record.
    if !namespace.children()?.is_empty() || open_record(&namespace.path.join(LIVE))?.is_some() {
        return Err(StorageError::TemporaryCleanupRequired);
    }
    if let Some(file) = open_record(&namespace.path.join(PENDING))? {
        dispose(&file)?;
    }
    Ok(())
}
fn dispose(file: &File) -> Result<(), StorageError> {
    let info = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: validated single-link private record and retained DELETE handle.
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
    Ok(())
}
fn publish(file: &File, path: &Path) -> Result<(), StorageError> {
    let mut name = wide(path)?;
    name.pop();
    let size = std::mem::offset_of!(FILE_RENAME_INFO, FileName) + name.len() * 2;
    let mut storage = vec![0u64; size.div_ceil(8)];
    let info = storage.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    // SAFETY: aligned bounded SDK buffer, same retained directory, no replacement.
    unsafe {
        (*info).Anonymous.Flags = 0;
        (*info).RootDirectory = null_mut();
        (*info).FileNameLength = (name.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(name.as_ptr(), (*info).FileName.as_mut_ptr(), name.len());
        if SetFileInformationByHandle(
            file.as_raw_handle(),
            FileRenameInfoEx,
            info.cast(),
            size as u32,
        ) == 0
        {
            return Err(last());
        }
    }
    Ok(())
}
