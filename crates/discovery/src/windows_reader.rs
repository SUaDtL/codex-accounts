//! Current-user registered-package reader. Name hints find candidates, not trust.
use crate::native::{Apartment, ReadHandle};
use crate::*;
use codex_accounts_core::compatibility::select;
use std::io::{Read, Seek, SeekFrom};
use windows::core::{HSTRING, Interface};
use windows::ApplicationModel::{Package, PackageSignatureKind};
use windows::Data::Xml::Dom::{XmlDocument, XmlElement, XmlLoadSettings};
use windows::Management::Deployment::PackageManager;
use windows::Security::Cryptography::{CryptographicBuffer};
use windows::Security::Cryptography::Core::HashAlgorithmProvider;

fn winerr(error: windows::core::Error) -> ReadError {
    if error.code().0 as u32 == 0x80070005 { ReadError::AccessDenied }
    else { ReadError::EnumerationUnavailable }
}
fn bounded(value: HSTRING) -> Result<String, ReadError> {
    if value.len() > MAX_PATH_UNITS { return Err(ReadError::InputLimit); }
    Ok(value.to_string_lossy())
}
fn hash_bytes(bytes: &[u8]) -> Result<String, ReadError> {
    let algorithm = HashAlgorithmProvider::OpenAlgorithm(&HSTRING::from("SHA256")).map_err(winerr)?;
    let buffer = CryptographicBuffer::CreateFromByteArray(bytes).map_err(winerr)?;
    let hash = algorithm.HashData(&buffer).map_err(winerr)?;
    bounded(CryptographicBuffer::EncodeToHexString(&hash).map_err(winerr)?)
}

struct Candidate {
    id: String,
    full_name: String,
    name: String,
    publisher: String,
    root: PathBuf,
    version: [u16; 4],
    architecture: i32,
    origin: &'static str,
}
impl fmt::Debug for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("Candidate([REDACTED])") }
}
impl Candidate {
    fn from_package(package: &Package) -> Result<Self, ReadError> {
        let identity = package.Id().map_err(winerr)?;
        let name = bounded(identity.Name().map_err(winerr)?)?;
        let full_name = bounded(identity.FullName().map_err(winerr)?)?;
        let publisher = bounded(identity.Publisher().map_err(winerr)?)?;
        let root = PathBuf::from(bounded(package.InstalledLocation().map_err(winerr)?.Path().map_err(winerr)?)?);
        let version = identity.Version().map_err(winerr)?;
        let kind = package.SignatureKind().map_err(winerr)?;
        let origin = match kind {
            PackageSignatureKind::Store => "registered_store_signature_kind",
            PackageSignatureKind::System => "registered_system_signature_kind",
            PackageSignatureKind::Enterprise => "registered_enterprise_signature_kind",
            PackageSignatureKind::Developer => "registered_developer_signature_kind",
            _ => "registration_signature_kind_unknown",
        };
        Ok(Self { id: hash_bytes(full_name.as_bytes())?, full_name, name, publisher, root,
            version: [version.Major, version.Minor, version.Build, version.Revision],
            architecture: identity.Architecture().map_err(winerr)?.0, origin })
    }
    fn report(&self, local: bool) -> Value {
        let mut result = json!({"candidate_id": self.id, "package_version": self.version,
            "package_architecture": self.architecture, "origin_observation": self.origin,
            "discovery_basis": "package_name_hint_only", "official_publisher": "not_reviewed",
            "qualified": false, "credential_mutation_enabled": false});
        if local {
            result["local_display_only_do_not_share"] = json!({"package_full_name": self.full_name,
                "package_name": self.name, "registration_publisher": self.publisher,
                "installed_location": path_text(&self.root)});
        }
        result
    }
}

fn candidates() -> Result<Vec<Candidate>, ReadError> {
    let manager = PackageManager::new().map_err(winerr)?;
    // Empty SID is the documented current-user query, not all users/elevation.
    let packages = manager.FindPackagesByUserSecurityId(&HSTRING::new()).map_err(winerr)?;
    let iterator = packages.First().map_err(winerr)?;
    let mut result = Vec::new();
    let mut count = 0;
    while iterator.HasCurrent().map_err(winerr)? {
        count += 1;
        if count > MAX_PACKAGES { return Err(ReadError::InputLimit); }
        let package = iterator.Current().map_err(winerr)?;
        let name = bounded(package.Id().map_err(winerr)?.Name().map_err(winerr)?)?;
        if is_candidate_hint(&name) && !package.IsFramework().map_err(winerr)? && !package.IsResourcePackage().map_err(winerr)? {
            if result.len() == MAX_CANDIDATES { return Err(ReadError::InputLimit); }
            result.push(Candidate::from_package(&package)?);
        }
        iterator.MoveNext().map_err(winerr)?;
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}

fn manifest_executables(root: &Path) -> Result<Vec<String>, ReadError> {
    let mut handle = ReadHandle::open(&root.join("AppxManifest.xml"), false)?;
    let bytes = handle.bytes(MAX_CONFIG_BYTES)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| ReadError::InvalidMetadata)?;
    let doc = XmlDocument::new().map_err(winerr)?;
    let settings = XmlLoadSettings::new().map_err(winerr)?;
    settings.SetProhibitDtd(true).map_err(winerr)?;
    settings.SetResolveExternals(false).map_err(winerr)?;
    settings.SetMaxElementDepth(64).map_err(winerr)?;
    doc.LoadXmlWithSettings(&HSTRING::from(text), &settings).map_err(|_| ReadError::InvalidMetadata)?;
    let nodes = doc.SelectNodes(&HSTRING::from("/*[local-name()='Package']/*[local-name()='Applications']/*[local-name()='Application']")).map_err(winerr)?;
    let count = nodes.Length().map_err(winerr)?;
    if count as usize > MAX_APPLICATIONS { return Err(ReadError::InputLimit); }
    let mut paths = Vec::new();
    for i in 0..count {
        let element: XmlElement = nodes.Item(i).map_err(winerr)?.cast().map_err(winerr)?;
        let executable = bounded(element.GetAttribute(&HSTRING::from("Executable")).map_err(winerr)?)?;
        if executable.is_empty() { return Err(ReadError::InvalidMetadata); }
        validate_relative_executable(&executable)?;
        if !paths.contains(&executable) { paths.push(executable); }
    }
    handle.revalidate()?;
    Ok(paths)
}

fn binary_observation(root: &Path, relative: &str, local: bool) -> Result<Value, ReadError> {
    validate_relative_executable(relative)?;
    let path = root.join(relative);
    let mut handle = ReadHandle::open(&path, false)?;
    let before = handle.identity();
    // Inspect only an on-disk PE image, never load it as a library or process.
    let mut dos = [0u8; 64];
    handle.file.read_exact(&mut dos).map_err(|_| ReadError::InvalidMetadata)?;
    if &dos[..2] != b"MZ" { return Err(ReadError::InvalidMetadata); }
    let offset = u32::from_le_bytes(dos[60..64].try_into().map_err(|_| ReadError::InvalidMetadata)?) as u64;
    if offset < 64 || offset > MAX_CONFIG_BYTES as u64 || offset + 6 > before.size { return Err(ReadError::InvalidMetadata); }
    handle.file.seek(SeekFrom::Start(offset)).map_err(|_| ReadError::Io)?;
    let mut pe = [0u8; 6];
    handle.file.read_exact(&mut pe).map_err(|_| ReadError::InvalidMetadata)?;
    if &pe[..4] != b"PE\0\0" { return Err(ReadError::InvalidMetadata); }
    let machine = u16::from_le_bytes([pe[4], pe[5]]);
    handle.file.seek(SeekFrom::Start(0)).map_err(|_| ReadError::Io)?;
    let algorithm = HashAlgorithmProvider::OpenAlgorithm(&HSTRING::from("SHA256")).map_err(winerr)?;
    let digest = algorithm.CreateHash().map_err(winerr)?;
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let n = handle.file.read(&mut buffer).map_err(|_| ReadError::Io)?;
        if n == 0 { break; }
        total += n as u64;
        if total > MAX_EXECUTABLE_BYTES { return Err(ReadError::InputLimit); }
        let block = CryptographicBuffer::CreateFromByteArray(&buffer[..n]).map_err(winerr)?;
        digest.Append(&block).map_err(winerr)?;
    }
    if total != before.size { return Err(ReadError::Changed); }
    let hash = bounded(CryptographicBuffer::EncodeToHexString(&digest.GetValueAndReset().map_err(winerr)?).map_err(winerr)?)?;
    let signature = if handle.cached_signature() { "cached_trust_accepted_not_official_identity" } else { "not_established_offline" };
    let writable = handle.current_user_can_replace_or_modify()?;
    handle.revalidate()?;
    let mut result = json!({"state": "observed", "sha256": hash, "size_bytes": total,
        "pe_machine": machine, "x64_image": machine == 0x8664,
        "signature": signature, "signature_mode": "cached_only_no_url_retrieval",
        "publisher_binding": "not_reviewed", "current_user_can_replace_or_modify": writable,
        "native_read_identity_stable": true, "file_version": "not_established",
        "path_protection": "read_handle_checks_not_complete_write_adapter",
        "file_identity": {"volume_serial": before.volume, "file_id": format!("{:016x}", before.file_id)},
        "qualified": false, "credential_mutation_enabled": false});
    if local { result["local_display_only_do_not_share"] = json!({"path": path_text(&path)}); }
    Ok(result)
}

fn config_observation(home: Option<&Path>) -> DeclaredConfig {
    let Some(home) = home else { return DeclaredConfig::unknown("not_nominated"); };
    let parent = match ReadHandle::open(home, true) {
        Ok(parent) => parent,
        Err(error) => return DeclaredConfig::unknown(error.code()),
    };
    let config = match ReadHandle::open(&home.join("config.toml"), false) {
        Ok(mut handle) => match handle.bytes(MAX_CONFIG_BYTES) {
            Ok(bytes) => parse_declared_config(&bytes),
            Err(error) => DeclaredConfig::unknown(error.code()),
        },
        Err(ReadError::Missing) => DeclaredConfig { state: "absent", backend: Observation::Absent, ..DeclaredConfig::unknown("absent") },
        Err(ReadError::AccessDenied) => DeclaredConfig { state: "inaccessible", backend: Observation::Inaccessible, ..DeclaredConfig::unknown("inaccessible") },
        Err(error) => DeclaredConfig::unknown(error.code()),
    };
    if parent.revalidate().is_err() { return DeclaredConfig::unknown("changed"); }
    config
}

pub fn inspect(opts: &Options) -> Result<Value, ReadError> {
    if codex_accounts_platform::current_lane() != codex_accounts_platform::DevelopmentLane::WindowsX64 {
        return Err(ReadError::UnsupportedPlatform);
    }
    let _apartment = Apartment::enter()?;
    let candidates = candidates()?;
    let ids: Vec<String> = candidates.iter().map(|c| c.id.clone()).collect();
    let mut report = json!({"schema": "codex-accounts/native-discovery/v1",
        "report_kind": if opts.local { "local_only_do_not_upload" } else { "sanitized_inventory_not_support_diagnostics" },
        "scope": "current_user_registered_main_packages_name_hints_only",
        "candidate_count": candidates.len(), "candidates": candidates.iter().map(|c| c.report(opts.local)).collect::<Vec<_>>(),
        "unpackaged_installations": "unsupported_not_searched", "official_publisher_reference": "not_reviewed",
        "qualified": false, "credential_mutation_enabled": false});
    let index = match select(&ids, opts.candidate.as_deref()) {
        Ok(index) => index,
        Err(reason) => {
            report["selection"] = json!(format!("{reason:?}"));
            report["outward_code"] = json!("E_COMPAT_UNKNOWN");
            return Ok(report);
        }
    };
    let selected = &candidates[index];
    // Hold the registered package root throughout inspection. No nominated
    // standalone executable can substitute for an installed application.
    let root = ReadHandle::open(&selected.root, true)?;
    let executables = manifest_executables(&selected.root)?;
    report["selected_candidate"] = json!(selected.id);
    report["manifest_application_count"] = json!(executables.len());
    if opts.local { report["local_manifest_executables_do_not_share"] = json!(executables); }
    if executables.len() == 1 {
        report["desktop"] = match binary_observation(&selected.root, &executables[0], opts.local) {
            Ok(observation) => observation, Err(error) => error_report(error),
        };
    } else {
        report["desktop"] = json!({"state": "ambiguous_or_missing_manifest_application", "binding": "unknown"});
    }
    report["runtime"] = match &opts.runtime_relative {
        Some(relative) => match binary_observation(&selected.root, relative, opts.local) {
            Ok(mut observation) => { observation["runtime_role"] = json!("nominated_in_package_not_qualified"); observation },
            Err(error) => error_report(error),
        },
        None => json!({"state": "not_nominated", "runtime_role": "unknown"}),
    };
    let config = config_observation(opts.home.as_deref());
    let flags: Vec<&str> = ENV_FLAGS.iter().copied().filter(|name| std::env::var_os(name).is_some()).collect();
    report["compatibility"] = context_report(&config, &flags);
    if opts.local { report["local_candidate_home_do_not_share"] = json!(opts.home.as_deref().map(path_text)); }
    root.revalidate()?;
    let after = self::candidates()?;
    let same = after.iter().find(|c| c.id == selected.id).is_some_and(|c| c.full_name == selected.full_name
        && c.root == selected.root && c.publisher == selected.publisher && c.version == selected.version
        && c.architecture == selected.architecture && c.origin == selected.origin);
    if !same { return Err(ReadError::Changed); }
    report["registration_stable_during_inspection"] = json!(true);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_current_user_inventory_is_bounded_and_not_qualification() {
        let result = inspect(&Options::default());
        match result {
            Ok(report) => {
                assert!(report["candidate_count"].as_u64().unwrap() <= MAX_CANDIDATES as u64);
                assert_eq!(report["credential_mutation_enabled"], false);
                assert_eq!(report["qualified"], false);
                assert!(!encode(&report).unwrap().contains("installed_location"));
            }
            Err(error @ (ReadError::AccessDenied | ReadError::EnumerationUnavailable)) => {
                // Native policy refusal is an observed outcome, not a package
                // inventory success. The test exercises its typed no-authority path.
                assert_eq!(error_report(error)["qualified"], false);
            }
            Err(error) => panic!("native inventory failed: {}", error.code()),
        }
    }
    #[test]
    fn native_hash_uses_standard_sha256() {
        let _apartment = Apartment::enter().unwrap();
        assert_eq!(hash_bytes(b"abc").unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
