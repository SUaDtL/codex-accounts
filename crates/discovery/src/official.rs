//! Current registered Store installation, pinned for one operation. This binds
//! packaged executables, not Desktop's effective home or runtime-selection policy.
use super::*;

const FAMILY: &str = "OpenAI.Codex_2p2nqsd0c76g0";
const PUBLISHER: &str = "CN=50BDFD77-8903-4850-9FFE-6E8522F64D5B";
const DESKTOP_NAMES: [&str; 2] = ["app/Codex.exe", "app/ChatGPT.exe"];
const RUNTIME: &str = "app/resources/codex.exe";
const PROVISIONING: &str = "microsoft.system.package.metadata/Custom.data";

struct Image {
    path: PathBuf,
    handle: ReadHandle,
    sha256: String,
}

/// Backend-only installation binding. Paths never enter serialized reports or
/// Debug output. Re-discovery accepts a new official version; an existing binding
/// refuses registration or file changes until the operation is restarted.
pub struct OfficialInstallation {
    candidate: Candidate,
    root: RegisteredRoot,
    manifest: ReadHandle,
    desktop: Image,
    runtime: Image,
    archive: ReadHandle,
    contained_core: bool,
    provisioning: Option<ReadHandle>,
    provisioned_core: bool,
}
impl fmt::Debug for OfficialInstallation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OfficialInstallation([REDACTED])")
    }
}

fn select_official(candidates: &[Candidate]) -> Result<&Candidate, ReadError> {
    let mut matching = candidates
        .iter()
        .filter(|candidate| candidate.family == FAMILY);
    let selected = matching.next().ok_or(ReadError::Missing)?;
    if matching.next().is_some() {
        return Err(ReadError::InvalidMetadata);
    }
    // Name hints only control inventory. Official selection requires the OS's
    // current Store registration and its exact stable family/publisher identity.
    if selected.name != "OpenAI.Codex"
        || selected.publisher != PUBLISHER
        || selected.origin != "registered_store_signature_kind"
    {
        return Err(ReadError::InvalidMetadata);
    }
    if selected.architecture != 9 {
        return Err(ReadError::UnsupportedPlatform);
    }
    Ok(selected)
}

fn desktop_relative(executables: &[String]) -> Result<&'static str, ReadError> {
    let mut selected = None;
    for executable in executables {
        if let Some(relative) = DESKTOP_NAMES
            .iter()
            .find(|relative| executable.replace('\\', "/").eq_ignore_ascii_case(relative))
        {
            if selected.replace(*relative).is_some() {
                return Err(ReadError::InvalidMetadata);
            }
        }
    }
    selected.ok_or(ReadError::InvalidMetadata)
}

fn official_signature(observation: crate::native::CachedSignature) -> bool {
    matches!(
        observation,
        crate::native::CachedSignature::AcceptedLeaf(bytes) if !bytes.is_empty()
    )
}

fn image(root: &RegisteredRoot, package: &Path, relative: &str) -> Result<Image, ReadError> {
    let mut handle = root.open_relative(relative)?;
    if handle.current_user_can_replace_or_modify()? {
        return Err(ReadError::UnsafePath);
    }
    if !official_signature(handle.cached_signature()) {
        return Err(ReadError::InvalidMetadata);
    }
    let path = package.join(relative);
    let identity = binary_identity(&mut handle)?;
    if identity.machine != 0x8664 {
        return Err(ReadError::UnsupportedPlatform);
    }
    Ok(Image {
        path,
        handle,
        sha256: identity.sha256,
    })
}

impl OfficialInstallation {
    /// Read current-user package metadata and the fixed manifest/executables.
    /// No process is launched and no Codex home or authentication file is opened.
    pub fn discover() -> Result<Self, ReadError> {
        if codex_accounts_platform::current_lane()
            != codex_accounts_platform::DevelopmentLane::WindowsX64
        {
            return Err(ReadError::UnsupportedPlatform);
        }
        let _apartment = Apartment::enter()?;
        let before = candidates_for_family(Some(FAMILY))?;
        let candidate = select_official(&before)?.clone();
        let root = RegisteredRoot::open(&candidate.root)?;
        if root.current_user_can_replace_or_modify()? {
            return Err(ReadError::UnsafePath);
        }
        let mut manifest = root.open_relative("AppxManifest.xml")?;
        if manifest.current_user_can_replace_or_modify()? {
            return Err(ReadError::UnsafePath);
        }
        let executables = parse_manifest(&manifest.bytes(MAX_CONFIG_BYTES)?, Some(&candidate))?;
        let desktop = image(&root, &candidate.root, desktop_relative(&executables)?)?;
        let runtime = image(&root, &candidate.root, RUNTIME)?;
        let mut archive = root.open_relative("app/resources/app.asar")?;
        if archive.current_user_can_replace_or_modify()? {
            return Err(ReadError::UnsafePath);
        }
        let contained_core = package_contained_core(&mut archive)?;
        let (provisioning, provisioned_core) = read_provisioning(&root)?;
        let installation = Self {
            candidate,
            root,
            manifest,
            desktop,
            runtime,
            archive,
            contained_core,
            provisioning,
            provisioned_core,
        };
        installation.revalidate_in_apartment()?;
        Ok(installation)
    }

    pub fn package_root(&self) -> &Path {
        &self.candidate.root
    }
    pub fn desktop_path(&self) -> &Path {
        &self.desktop.path
    }
    pub fn runtime_path(&self) -> &Path {
        &self.runtime.path
    }
    pub fn contained_core(&self) -> bool {
        self.contained_core
    }
    /// Desktop's provisioning object enables its runtime-framework defaults.
    /// No partner, deal, timestamp, or other metadata leaves this binding.
    pub fn provisioned_core(&self) -> bool {
        self.provisioned_core
    }
    pub fn version(&self) -> [u16; 4] {
        self.candidate.version
    }
    /// Observed image hashes identify this operation, never a version allowlist.
    pub fn desktop_sha256(&self) -> &str {
        &self.desktop.sha256
    }
    pub fn runtime_sha256(&self) -> &str {
        &self.runtime.sha256
    }

    pub fn revalidate(&self) -> Result<(), ReadError> {
        let _apartment = Apartment::enter()?;
        self.revalidate_in_apartment()
    }
    fn same_registration(&self) -> Result<(), ReadError> {
        let current = candidates_for_family(Some(FAMILY))?;
        if select_official(&current).map_err(|_| ReadError::Changed)? != &self.candidate {
            return Err(ReadError::Changed);
        }
        Ok(())
    }
    fn revalidate_in_apartment(&self) -> Result<(), ReadError> {
        self.same_registration()?;
        self.root.revalidate()?;
        for handle in [
            &self.manifest,
            &self.desktop.handle,
            &self.runtime.handle,
            &self.archive,
        ] {
            handle.revalidate()?;
            if handle.current_user_can_replace_or_modify()? {
                return Err(ReadError::UnsafePath);
            }
        }
        if self.root.current_user_can_replace_or_modify()? {
            return Err(ReadError::UnsafePath);
        }
        if let Some(handle) = &self.provisioning {
            handle.revalidate()?;
            if handle.current_user_can_replace_or_modify()? {
                return Err(ReadError::UnsafePath);
            }
        } else {
            match self.root.open_relative(PROVISIONING) {
                Err(ReadError::Missing) => {}
                Ok(_) => return Err(ReadError::Changed),
                Err(error) => return Err(error),
            }
        }
        self.same_registration()
    }
}

fn read_provisioning(root: &RegisteredRoot) -> Result<(Option<ReadHandle>, bool), ReadError> {
    let mut handle = match root.open_relative(PROVISIONING) {
        Ok(handle) => handle,
        Err(ReadError::Missing) => return Ok((None, false)),
        Err(error) => return Err(error),
    };
    if handle.current_user_can_replace_or_modify()? {
        return Err(ReadError::UnsafePath);
    }
    // Match Desktop dk(): at most 4096 bytes, optional UTF-8 BOM, any JSON
    // object. Its optional partner/deal/provisioned_on fields catch type errors;
    // even {} yields non-null and enables both framework defaults in fT().
    let provisioned = match handle.bytes(4096) {
        Ok(bytes) => provisioning_object(&bytes),
        Err(ReadError::InputLimit) => false,
        Err(error) => return Err(error),
    };
    Ok((Some(handle), provisioned))
}
fn provisioning_object(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    matches!(serde_json::from_slice::<Value>(bytes), Ok(Value::Object(_)))
}

fn package_contained_core(archive: &mut ReadHandle) -> Result<bool, ReadError> {
    let prefix = archive.range(0, 16)?;
    let integer = |offset| {
        u32::from_le_bytes(prefix[offset..offset + 4].try_into().expect("fixed prefix")) as usize
    };
    let header_bytes = integer(4);
    let json_bytes = integer(12);
    if integer(0) != 4
        || !(8..=8 * MAX_CONFIG_BYTES).contains(&header_bytes)
        || json_bytes > header_bytes - 8
    {
        return Err(ReadError::InvalidMetadata);
    }
    let header: Value = serde_json::from_slice(&archive.range(16, json_bytes)?)
        .map_err(|_| ReadError::InvalidMetadata)?;
    let entry = &header["files"]["package.json"];
    if entry.get("unpacked").is_some() || entry.get("link").is_some() {
        return Err(ReadError::InvalidMetadata);
    }
    let offset: u64 = entry["offset"]
        .as_str()
        .ok_or(ReadError::InvalidMetadata)?
        .parse()
        .map_err(|_| ReadError::InvalidMetadata)?;
    let size = entry["size"]
        .as_u64()
        .filter(|v| *v <= MAX_CONFIG_BYTES as u64)
        .ok_or(ReadError::InputLimit)?;
    let data_offset = 8u64
        .checked_add(header_bytes as u64)
        .and_then(|n| n.checked_add(offset))
        .ok_or(ReadError::InputLimit)?;
    let package: Value = serde_json::from_slice(&archive.range(data_offset, size as usize)?)
        .map_err(|_| ReadError::InvalidMetadata)?;
    match package.get("codexWindowsAppContainedCore") {
        None => Ok(false),
        Some(Value::String(value)) if value == "1" => Ok(true),
        Some(Value::String(value)) if value == "0" => Ok(false),
        _ => Err(ReadError::InvalidMetadata),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lifecycle_images_require_cached_authenticode_with_a_signer() {
        use crate::native::CachedSignature;
        assert!(official_signature(CachedSignature::AcceptedLeaf(vec![1])));
        assert!(!official_signature(CachedSignature::NotEstablished));
        assert!(!official_signature(
            CachedSignature::AcceptedSignerUnavailable
        ));
        assert!(!official_signature(CachedSignature::CleanupFailed));
    }

    #[test]
    fn provisioning_matches_desktop_optional_object_semantics() {
        assert!(provisioning_object(b"{}"));
        assert!(provisioning_object(
            b"\xef\xbb\xbf{\"partner\":null,\"provisioned_on\":\"bad-type\"}"
        ));
        for bytes in [
            b"null".as_slice(),
            b"[]",
            b"false",
            b"\"text\"",
            b"{",
            b"\xff",
        ] {
            assert!(!provisioning_object(bytes));
        }
    }
    fn candidate(version: [u16; 4]) -> Candidate {
        Candidate {
            id: "SYNTHETIC".into(),
            full_name: "SYNTHETIC".into(),
            family: FAMILY.into(),
            name: "OpenAI.Codex".into(),
            publisher: PUBLISHER.into(),
            root: PathBuf::from("C:/SYNTHETIC"),
            version,
            architecture: 9,
            origin: "registered_store_signature_kind",
        }
    }
    #[test]
    fn official_selection_accepts_updates_but_rejects_identity_and_origin_substitutions() {
        for relative in DESKTOP_NAMES {
            assert_eq!(
                desktop_relative(&[
                    relative.into(),
                    "app/resources/codex-command-runner.exe".into()
                ])
                .unwrap(),
                relative
            );
        }
        assert!(desktop_relative(&[]).is_err());
        assert!(desktop_relative(&DESKTOP_NAMES.map(String::from)).is_err());
        for version in [[1, 0, 0, 0], [99, 999, 5, 7]] {
            assert_eq!(
                select_official(&[candidate(version)]).unwrap().version,
                version
            );
        }
        let valid = candidate([1, 2, 3, 4]);
        assert_eq!(select_official(&[]).unwrap_err(), ReadError::Missing);
        assert_eq!(
            select_official(&[valid.clone(), valid.clone()]).unwrap_err(),
            ReadError::InvalidMetadata
        );
        for field in ["family", "publisher", "name", "origin"] {
            let mut changed = valid.clone();
            match field {
                "family" => changed.family = "Fake.Codex_name_hint".into(),
                "publisher" => changed.publisher = "CN=SYNTHETIC".into(),
                "name" => changed.name = "Fake.Codex".into(),
                _ => changed.origin = "registered_developer_signature_kind",
            }
            assert!(select_official(&[changed]).is_err());
        }
        let mut changed = valid;
        changed.architecture = 12;
        assert_eq!(
            select_official(&[changed]).unwrap_err(),
            ReadError::UnsupportedPlatform
        );
    }

    #[test]
    #[ignore = "current-user package metadata only; compares exact-family lookup with inventory"]
    fn native_official_family_query_matches_broad_inventory() {
        let _apartment = Apartment::enter().unwrap();
        let broad: Vec<_> = candidates()
            .unwrap()
            .into_iter()
            .filter(|candidate| candidate.family == FAMILY)
            .collect();
        let narrow = candidates_for_family(Some(FAMILY)).unwrap();
        assert!(
            !broad.is_empty(),
            "official package must be present for this check"
        );
        assert_eq!(broad, narrow);
        select_official(&narrow).unwrap();
    }

    #[test]
    #[ignore = "requires the official Store package installed for the current user"]
    fn native_official_installation_retains_current_packaged_images() {
        let installation = OfficialInstallation::discover().unwrap();
        assert_eq!(
            format!("{installation:?}"),
            "OfficialInstallation([REDACTED])"
        );
        assert_eq!(installation.desktop_sha256().len(), 64);
        assert_eq!(installation.runtime_sha256().len(), 64);
        assert!(DESKTOP_NAMES
            .iter()
            .any(|relative| installation.desktop_path()
                == installation.package_root().join(relative)));
        assert_eq!(
            installation.runtime_path(),
            installation.package_root().join(RUNTIME)
        );
        installation.revalidate().unwrap();
    }
}
