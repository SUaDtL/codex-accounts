//! Current-user registered-package reader. Name hints find candidates, not trust.
use crate::native::{Apartment, ReadHandle, RegisteredRoot};
use crate::*;
use codex_accounts_core::compatibility::select;
use std::io::{Read, Seek, SeekFrom};
use windows::core::{Interface, HSTRING};
use windows::ApplicationModel::{Package, PackageSignatureKind};
use windows::Data::Xml::Dom::{XmlDocument, XmlElement, XmlLoadSettings};
use windows::Management::Deployment::PackageManager;
use windows::Security::Cryptography::Core::HashAlgorithmProvider;
use windows::Security::Cryptography::CryptographicBuffer;

fn winerr(error: windows::core::Error) -> ReadError {
    if error.code().0 as u32 == 0x80070005 {
        ReadError::AccessDenied
    } else {
        ReadError::EnumerationUnavailable
    }
}
fn bounded(value: HSTRING) -> Result<String, ReadError> {
    if value.len() > MAX_PATH_UNITS {
        return Err(ReadError::InputLimit);
    }
    String::from_utf16(&value).map_err(|_| ReadError::InvalidMetadata)
}
fn hash_bytes(bytes: &[u8]) -> Result<String, ReadError> {
    let algorithm =
        HashAlgorithmProvider::OpenAlgorithm(&HSTRING::from("SHA256")).map_err(winerr)?;
    let buffer = CryptographicBuffer::CreateFromByteArray(bytes).map_err(winerr)?;
    let hash = algorithm.HashData(&buffer).map_err(winerr)?;
    bounded(CryptographicBuffer::EncodeToHexString(&hash).map_err(winerr)?)
}

fn signature_report(
    observation: crate::native::CachedSignature,
) -> Result<(&'static str, Value), ReadError> {
    use crate::native::CachedSignature;
    let (signature, state, leaf_hash) = match observation {
        CachedSignature::NotEstablished => {
            ("not_established_offline", "not_established_offline", None)
        }
        CachedSignature::AcceptedSignerUnavailable => (
            "cached_trust_accepted_not_official_identity",
            "signer_unavailable",
            None,
        ),
        CachedSignature::AcceptedLeaf(bytes) => (
            "cached_trust_accepted_not_official_identity",
            "observed_not_publisher_approval",
            Some(hash_bytes(&bytes)?),
        ),
        CachedSignature::CleanupFailed => {
            ("not_established_offline", "cleanup_not_established", None)
        }
    };
    Ok((
        signature,
        json!({"state": state, "leaf_der_sha256": leaf_hash}),
    ))
}

#[derive(Clone, PartialEq, Eq)]
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Candidate([REDACTED])")
    }
}
impl Candidate {
    fn from_package(package: &Package) -> Result<Self, ReadError> {
        let identity = package.Id().map_err(winerr)?;
        let name = bounded(identity.Name().map_err(winerr)?)?;
        let full_name = bounded(identity.FullName().map_err(winerr)?)?;
        let publisher = bounded(identity.Publisher().map_err(winerr)?)?;
        let root = PathBuf::from(bounded(
            package
                .InstalledLocation()
                .map_err(winerr)?
                .Path()
                .map_err(winerr)?,
        )?);
        let version = identity.Version().map_err(winerr)?;
        let kind = package.SignatureKind().map_err(winerr)?;
        let origin = match kind {
            PackageSignatureKind::Store => "registered_store_signature_kind",
            PackageSignatureKind::System => "registered_system_signature_kind",
            PackageSignatureKind::Enterprise => "registered_enterprise_signature_kind",
            PackageSignatureKind::Developer => "registered_developer_signature_kind",
            _ => "registration_signature_kind_unknown",
        };
        Ok(Self {
            id: hash_bytes(full_name.as_bytes())?,
            full_name,
            name,
            publisher,
            root,
            version: [
                version.Major,
                version.Minor,
                version.Build,
                version.Revision,
            ],
            architecture: identity.Architecture().map_err(winerr)?.0,
            origin,
        })
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
    let packages = manager
        .FindPackagesByUserSecurityId(&HSTRING::new())
        .map_err(winerr)?;
    let iterator = packages.First().map_err(winerr)?;
    let mut result = Vec::new();
    let mut count = 0;
    while iterator.HasCurrent().map_err(winerr)? {
        count += 1;
        if count > MAX_PACKAGES {
            return Err(ReadError::InputLimit);
        }
        let package = iterator.Current().map_err(winerr)?;
        let name = bounded(package.Id().map_err(winerr)?.Name().map_err(winerr)?)?;
        if is_candidate_hint(&name)
            && !package.IsFramework().map_err(winerr)?
            && !package.IsResourcePackage().map_err(winerr)?
        {
            if result.len() == MAX_CANDIDATES {
                return Err(ReadError::InputLimit);
            }
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
    let paths = parse_manifest(&bytes, None)?;
    handle.revalidate()?;
    Ok(paths)
}

fn parse_manifest(bytes: &[u8], registered: Option<&Candidate>) -> Result<Vec<String>, ReadError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ReadError::InvalidMetadata)?;
    // WinRT takes decoded XML text, so remove only the optional encoding marker.
    // Validate UTF-8 first; another marker or malformed prefix remains invalid XML.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let doc = XmlDocument::new().map_err(winerr)?;
    let settings = XmlLoadSettings::new().map_err(winerr)?;
    settings.SetProhibitDtd(true).map_err(winerr)?;
    settings.SetResolveExternals(false).map_err(winerr)?;
    settings.SetMaxElementDepth(64).map_err(winerr)?;
    doc.LoadXmlWithSettings(&HSTRING::from(text), &settings)
        .map_err(|_| ReadError::InvalidMetadata)?;
    if let Some(candidate) = registered {
        let identities = doc
            .SelectNodes(&HSTRING::from(
                "/*[local-name()='Package']/*[local-name()='Identity']",
            ))
            .map_err(winerr)?;
        if identities.Length().map_err(winerr)? != 1 {
            return Err(ReadError::InvalidMetadata);
        }
        let identity: XmlElement = identities.Item(0).map_err(winerr)?.cast().map_err(winerr)?;
        let attr = |name| {
            bounded(
                identity
                    .GetAttribute(&HSTRING::from(name))
                    .map_err(winerr)?,
            )
        };
        let architecture = match candidate.architecture {
            0 => "x86",
            5 => "arm",
            9 => "x64",
            11 => "neutral",
            12 => "arm64",
            _ => return Err(ReadError::InvalidMetadata),
        };
        let version = candidate.version.map(|n| n.to_string()).join(".");
        if attr("Name")? != candidate.name
            || attr("Publisher")? != candidate.publisher
            || attr("Version")? != version
            || attr("ProcessorArchitecture")? != architecture
        {
            return Err(ReadError::InvalidMetadata);
        }
    }
    let nodes = doc.SelectNodes(&HSTRING::from("/*[local-name()='Package']/*[local-name()='Applications']/*[local-name()='Application']")).map_err(winerr)?;
    let count = nodes.Length().map_err(winerr)?;
    if count as usize > MAX_APPLICATIONS {
        return Err(ReadError::InputLimit);
    }
    let mut paths = Vec::new();
    for i in 0..count {
        let element: XmlElement = nodes.Item(i).map_err(winerr)?.cast().map_err(winerr)?;
        let executable = bounded(
            element
                .GetAttribute(&HSTRING::from("Executable"))
                .map_err(winerr)?,
        )?;
        if executable.is_empty() {
            return Err(ReadError::InvalidMetadata);
        }
        validate_relative_executable(&executable)?;
        if !paths.contains(&executable) {
            paths.push(executable);
        }
    }
    Ok(paths)
}

fn binary_observation(
    root: &Path,
    relative: &str,
    local: bool,
) -> Result<(Value, ReadHandle), ReadError> {
    validate_relative_executable(relative)?;
    let path = root.join(relative);
    let mut handle = ReadHandle::open(&path, false)?;
    binary_from_handle(&mut handle, &path, local, false).map(|observation| (observation, handle))
}

fn binary_from_handle(
    handle: &mut ReadHandle,
    path: &Path,
    local: bool,
    partial: bool,
) -> Result<Value, ReadError> {
    let before = handle.identity();
    // Inspect only an on-disk PE image, never load it as a library or process.
    let mut dos = [0u8; 64];
    handle
        .file
        .read_exact(&mut dos)
        .map_err(|_| ReadError::InvalidMetadata)?;
    if &dos[..2] != b"MZ" {
        return Err(ReadError::InvalidMetadata);
    }
    let offset = u32::from_le_bytes(
        dos[60..64]
            .try_into()
            .map_err(|_| ReadError::InvalidMetadata)?,
    ) as u64;
    if offset < 64 || offset > MAX_CONFIG_BYTES as u64 || offset + 6 > before.size {
        return Err(ReadError::InvalidMetadata);
    }
    handle
        .file
        .seek(SeekFrom::Start(offset))
        .map_err(|_| ReadError::Io)?;
    let mut pe = [0u8; 6];
    handle
        .file
        .read_exact(&mut pe)
        .map_err(|_| ReadError::InvalidMetadata)?;
    if &pe[..4] != b"PE\0\0" {
        return Err(ReadError::InvalidMetadata);
    }
    let machine = u16::from_le_bytes([pe[4], pe[5]]);
    handle
        .file
        .seek(SeekFrom::Start(0))
        .map_err(|_| ReadError::Io)?;
    let algorithm =
        HashAlgorithmProvider::OpenAlgorithm(&HSTRING::from("SHA256")).map_err(winerr)?;
    let digest = algorithm.CreateHash().map_err(winerr)?;
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let n = handle.file.read(&mut buffer).map_err(|_| ReadError::Io)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > MAX_EXECUTABLE_BYTES {
            return Err(ReadError::InputLimit);
        }
        let block = CryptographicBuffer::CreateFromByteArray(&buffer[..n]).map_err(winerr)?;
        digest.Append(&block).map_err(winerr)?;
    }
    if total != before.size {
        return Err(ReadError::Changed);
    }
    let hash = bounded(
        CryptographicBuffer::EncodeToHexString(&digest.GetValueAndReset().map_err(winerr)?)
            .map_err(winerr)?,
    )?;
    let (signature, primary_signer) = signature_report(handle.cached_signature())?;
    let writable = if partial {
        Value::String("unknown".into())
    } else {
        json!(handle.current_user_can_replace_or_modify()?)
    };
    handle.revalidate()?;
    let mut result = json!({"state": "observed", "sha256": hash, "size_bytes": total,
        "pe_machine": machine, "x64_image": machine == 0x8664,
        "signature": signature, "signature_mode": "cached_only_no_url_retrieval",
        "primary_signer": primary_signer,
        "publisher_binding": "not_reviewed", "current_user_can_replace_or_modify": writable,
        "native_read_identity_stable": true, "file_version": "not_established",
        "path_protection": "read_handle_checks_not_complete_write_adapter",
        "file_identity": {"volume_serial": before.volume, "file_id": format!("{:016x}", before.file_id)},
        "qualified": false, "credential_mutation_enabled": false});
    if local {
        result["local_display_only_do_not_share"] = json!({"path": path_text(path)});
    }
    if partial {
        result["path_protection"] = json!("partial_registered_root_and_descendants_only");
        result["outer_ancestry_validation"] = json!("not_established");
        result["executable_role"] = json!("unqualified_manifest_candidate");
    }
    Ok(result)
}

#[cfg(test)]
fn collect_registered(
    selected: &Candidate,
    local: bool,
    reobserve: impl FnOnce() -> Result<Vec<Candidate>, ReadError>,
) -> Result<Value, ReadError> {
    collect_registered_image(selected, local, None, reobserve)
}

fn collect_registered_image(
    selected: &Candidate,
    local: bool,
    nominated: Option<&str>,
    reobserve: impl FnOnce() -> Result<Vec<Candidate>, ReadError>,
) -> Result<Value, ReadError> {
    if let Some(relative) = nominated {
        validate_relative_executable(relative)?;
    }
    let root = RegisteredRoot::open(&selected.root)?;
    let mut manifest = root.open_relative("AppxManifest.xml")?;
    let bytes = manifest.bytes(MAX_CONFIG_BYTES)?;
    let executables = parse_manifest(&bytes, Some(selected))?;
    let mut handles = Vec::new();
    let mut images = Vec::new();
    for relative in &executables {
        let mut image = root.open_relative(relative)?;
        images.push(binary_from_handle(
            &mut image,
            &selected.root.join(relative),
            local,
            true,
        )?);
        handles.push(image);
    }
    let nominated_image = if let Some(relative) = nominated {
        let mut image = root.open_relative(relative)?;
        let mut observation =
            binary_from_handle(&mut image, &selected.root.join(relative), local, true)?;
        observation["provenance"] = json!("caller_nominated_package_relative");
        observation["executable_role"] = json!("unqualified_nominated_image");
        handles.push(image);
        Some(observation)
    } else {
        None
    };
    if reobserve()?
        .iter()
        .filter(|c| c.id == selected.id)
        .collect::<Vec<_>>()
        != vec![selected]
    {
        return Err(ReadError::Changed);
    }
    root.revalidate()?;
    manifest.revalidate()?;
    for image in &handles {
        image.revalidate()?;
    }
    let mut report = json!({"observation_scope": "partial_registered_root_and_declared_images",
        "outer_ancestry_validation": "not_established", "registration_stable_during_inspection": true,
        "manifest_identity_matches_registration": true, "manifest_application_count": executables.len(),
        "images": images, "publisher_binding": "unknown", "runtime_role": "unknown",
        "effective_home": "unknown", "effective_backend": "unknown", "effective_policy": "unknown",
        "qualified": false, "credential_mutation_enabled": false});
    if let Some(observation) = nominated_image {
        report["nominated_image"] = observation;
        report["observation_scope"] =
            json!("partial_registered_root_declared_and_nominated_images");
    }
    Ok(report)
}

fn config_observation(home: Option<&Path>) -> DeclaredConfig {
    let Some(home) = home else {
        return DeclaredConfig::unknown("not_nominated");
    };
    let parent = match ReadHandle::open(home, true) {
        Ok(parent) => parent,
        Err(error) => return DeclaredConfig::unknown(error.code()),
    };
    let config = match ReadHandle::open(&home.join("config.toml"), false) {
        Ok(mut handle) => match handle.bytes(MAX_CONFIG_BYTES) {
            Ok(bytes) => parse_declared_config(&bytes),
            Err(error) => DeclaredConfig::unknown(error.code()),
        },
        Err(ReadError::Missing) => DeclaredConfig {
            state: "absent",
            backend: Observation::Absent,
            ..DeclaredConfig::unknown("absent")
        },
        Err(ReadError::AccessDenied) => DeclaredConfig {
            state: "inaccessible",
            backend: Observation::Inaccessible,
            ..DeclaredConfig::unknown("inaccessible")
        },
        Err(error) => DeclaredConfig::unknown(error.code()),
    };
    if parent.revalidate().is_err() {
        return DeclaredConfig::unknown("changed");
    }
    config
}

pub fn inspect(opts: &Options) -> Result<Value, ReadError> {
    if codex_accounts_platform::current_lane()
        != codex_accounts_platform::DevelopmentLane::WindowsX64
    {
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
    if opts.registered_metadata {
        report["registered_metadata"] = collect_registered_image(
            selected,
            opts.local,
            opts.registered_image.as_deref(),
            self::candidates,
        )?;
        return Ok(report);
    }
    // Hold the registered package root throughout inspection. No nominated
    // standalone executable can substitute for an installed application.
    let root = ReadHandle::open(&selected.root, true)?;
    let executables = manifest_executables(&selected.root)?;
    report["selected_candidate"] = json!(selected.id);
    report["manifest_application_count"] = json!(executables.len());
    if opts.local {
        report["local_manifest_executables_do_not_share"] = json!(executables);
    }
    let mut images = Vec::new();
    if executables.len() == 1 {
        report["desktop"] = match binary_observation(&selected.root, &executables[0], opts.local) {
            Ok((observation, image)) => {
                images.push(image);
                observation
            }
            Err(error) => error_report(error),
        };
    } else {
        report["desktop"] =
            json!({"state": "ambiguous_or_missing_manifest_application", "binding": "unknown"});
    }
    report["runtime"] = match &opts.runtime_relative {
        Some(relative) => match binary_observation(&selected.root, relative, opts.local) {
            Ok((mut observation, image)) => {
                images.push(image);
                observation["runtime_role"] = json!("nominated_in_package_not_qualified");
                observation
            }
            Err(error) => error_report(error),
        },
        None => json!({"state": "not_nominated", "runtime_role": "unknown"}),
    };
    let config = config_observation(opts.home.as_deref());
    let flags: Vec<&str> = ENV_FLAGS
        .iter()
        .copied()
        .filter(|name| std::env::var_os(name).is_some())
        .collect();
    report["compatibility"] = context_report(&config, &flags);
    if opts.local {
        report["local_candidate_home_do_not_share"] = json!(opts.home.as_deref().map(path_text));
    }
    root.revalidate()?;
    let after = self::candidates()?;
    let same = after.iter().find(|c| c.id == selected.id).is_some_and(|c| {
        c.full_name == selected.full_name
            && c.root == selected.root
            && c.publisher == selected.publisher
            && c.version == selected.version
            && c.architecture == selected.architecture
            && c.origin == selected.origin
    });
    if !same {
        return Err(ReadError::Changed);
    }
    for image in &images {
        image.revalidate()?;
    }
    report["registration_stable_during_inspection"] = json!(true);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT_METADATA: AtomicUsize = AtomicUsize::new(0);
    struct MetadataFixture(Candidate);
    impl MetadataFixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "ca-metadata-SYNTHETIC_CANARY-{}-{}",
                std::process::id(),
                NEXT_METADATA.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            let candidate = Candidate {
                id: "a".repeat(64),
                full_name: "Synthetic.Codex_1.2.3.4_x64_fixture".into(),
                name: "Synthetic.Codex".into(),
                publisher: "CN=Synthetic".into(),
                root,
                version: [1, 2, 3, 4],
                architecture: 9,
                origin: "registration_signature_kind_unknown",
            };
            let fixture = Self(candidate);
            fixture.manifest(
                "<Application Executable='first.exe'/><Application Executable='second.exe'/>",
            );
            let mut pe = vec![0u8; 512];
            pe[..2].copy_from_slice(b"MZ");
            pe[60..64].copy_from_slice(&128u32.to_le_bytes());
            pe[128..132].copy_from_slice(b"PE\0\0");
            pe[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
            for file in ["first.exe", "second.exe"] {
                std::fs::write(fixture.0.root.join(file), &pe).unwrap();
            }
            fixture
        }
        fn manifest(&self, apps: &str) {
            std::fs::write(self.0.root.join("AppxManifest.xml"), format!("<Package><Identity Name='Synthetic.Codex' Publisher='CN=Synthetic' Version='1.2.3.4' ProcessorArchitecture='x64'/><Applications>{apps}</Applications></Package>")).unwrap();
        }
        fn collect(&self) -> Result<Value, ReadError> {
            collect_registered(&self.0, false, || Ok(vec![self.0.clone()]))
        }
    }
    impl Drop for MetadataFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0.root);
        }
    }

    #[test]
    fn native_registered_image_is_separate_redacted_and_retained_through_reobservation() {
        use std::os::windows::fs::OpenOptionsExt;
        let _apartment = Apartment::enter().unwrap();
        let fixture = MetadataFixture::new();
        let relative = "images/SYNTHETIC_NOMINATED_CANARY.exe";
        std::fs::create_dir(fixture.0.root.join("images")).unwrap();
        std::fs::copy(
            fixture.0.root.join("first.exe"),
            fixture.0.root.join(relative),
        )
        .unwrap();
        let mut guards = Vec::new();
        for file in ["config.toml", "auth.json", "history.jsonl"] {
            std::fs::write(fixture.0.root.join(file), b"SYNTHETIC_NEVER_READ").unwrap();
            guards.push(
                std::fs::OpenOptions::new()
                    .read(true)
                    .share_mode(0)
                    .open(fixture.0.root.join(file))
                    .unwrap(),
            );
        }
        let original = fixture.collect().unwrap();
        crate::native::take_opened_paths();
        let report = collect_registered_image(&fixture.0, false, Some(relative), || {
            for file in ["AppxManifest.xml", "first.exe", "second.exe", relative] {
                assert!(std::fs::OpenOptions::new()
                    .write(true)
                    .open(fixture.0.root.join(file))
                    .is_err());
                assert!(std::fs::rename(
                    fixture.0.root.join(file),
                    fixture.0.root.join("substitution.exe")
                )
                .is_err());
            }
            assert!(std::fs::rename(
                fixture.0.root.join("images"),
                fixture.0.root.join("replaced")
            )
            .is_err());
            Ok(vec![fixture.0.clone()])
        })
        .unwrap();
        assert_eq!(report["images"], original["images"]);
        assert_eq!(report["images"].as_array().unwrap().len(), 2);
        assert_eq!(
            report["nominated_image"]["provenance"],
            "caller_nominated_package_relative"
        );
        assert_eq!(
            report["nominated_image"]["executable_role"],
            "unqualified_nominated_image"
        );
        assert_eq!(
            report["nominated_image"]["current_user_can_replace_or_modify"],
            "unknown"
        );
        assert_eq!(report["nominated_image"]["qualified"], false);
        assert_eq!(
            report["nominated_image"]["credential_mutation_enabled"],
            false
        );
        for field in [
            "publisher_binding",
            "runtime_role",
            "effective_home",
            "effective_backend",
            "effective_policy",
        ] {
            assert_eq!(report[field], "unknown");
        }
        assert_eq!(report["outer_ancestry_validation"], "not_established");
        assert!(report.get("runtime").is_none());
        assert!(!encode(&report).unwrap().contains("SYNTHETIC"));
        let opened = crate::native::take_opened_paths();
        assert!(opened.iter().any(|p| p == &fixture.0.root.join(relative)));
        assert!(opened.iter().all(|p| p.is_dir()
            || [
                "AppxManifest.xml",
                "first.exe",
                "second.exe",
                "SYNTHETIC_NOMINATED_CANARY.exe"
            ]
            .iter()
            .any(|name| p.file_name().is_some_and(|f| f == *name))));
        let local = collect_registered_image(&fixture.0, true, Some(relative), || {
            Ok(vec![fixture.0.clone()])
        })
        .unwrap();
        assert!(
            local["nominated_image"]["local_display_only_do_not_share"]["path"]
                .as_str()
                .unwrap()
                .contains("SYNTHETIC_NOMINATED_CANARY")
        );
        drop(guards);
        for file in ["config.toml", "auth.json", "history.jsonl"] {
            assert_eq!(
                std::fs::read(fixture.0.root.join(file)).unwrap(),
                b"SYNTHETIC_NEVER_READ"
            );
        }
    }

    #[test]
    fn native_registered_image_rejects_junction_ancestor_without_target_read() {
        let _apartment = Apartment::enter().unwrap();
        let fixture = MetadataFixture::new();
        let outside = MetadataFixture::new();
        let junction = fixture.0.root.join("images");
        std::fs::create_dir(&junction).unwrap();
        crate::native::tests::install_synthetic_junction(&junction, &outside.0.root);
        crate::native::take_opened_paths();
        let result = collect_registered_image(&fixture.0, false, Some("images/first.exe"), || {
            Ok(vec![fixture.0.clone()])
        });
        let opened = crate::native::take_opened_paths();
        std::fs::remove_dir(&junction).unwrap();
        assert!(matches!(result, Err(ReadError::UnsafePath)));
        assert!(!opened
            .iter()
            .any(|path| path.starts_with(&outside.0.root) || path == &junction.join("first.exe")));
        assert!(outside.0.root.join("first.exe").is_file());
    }

    #[test]
    fn native_registered_image_refuses_invalid_leaf_and_registration_changes() {
        let _apartment = Apartment::enter().unwrap();
        let fixture = MetadataFixture::new();
        let relative = "images/nominated.exe";
        std::fs::create_dir(fixture.0.root.join("images")).unwrap();
        let path = fixture.0.root.join(relative);
        let collect = || {
            collect_registered_image(&fixture.0, false, Some(relative), || {
                Ok(vec![fixture.0.clone()])
            })
        };
        assert!(matches!(collect(), Err(ReadError::Missing)));
        std::fs::write(&path, b"SYNTHETIC_INVALID_PE").unwrap();
        assert!(matches!(collect(), Err(ReadError::InvalidMetadata)));
        std::fs::copy(fixture.0.root.join("first.exe"), &path).unwrap();
        let writer = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        assert!(matches!(collect(), Err(ReadError::SharingViolation)));
        drop(writer);
        let alias = fixture.0.root.join("alias.exe");
        std::fs::hard_link(&path, &alias).unwrap();
        assert!(matches!(collect(), Err(ReadError::UnsafePath)));
        std::fs::remove_file(alias).unwrap();
        let mut changed = fixture.0.clone();
        changed.version[0] += 1;
        assert!(matches!(
            collect_registered_image(&fixture.0, false, Some(relative), || Ok(vec![changed])),
            Err(ReadError::Changed)
        ));
        let mut mismatched = fixture.0.clone();
        mismatched.publisher.push('x');
        assert!(matches!(
            collect_registered_image(&mismatched, false, Some(relative), || Ok(vec![
                mismatched.clone()
            ])),
            Err(ReadError::InvalidMetadata)
        ));
        assert!(matches!(
            collect_registered_image(&fixture.0, false, Some("../outside.exe"), || Ok(vec![
                fixture.0.clone()
            ])),
            Err(ReadError::UnsafePath)
        ));
    }

    #[test]
    fn native_registered_metadata_observes_each_declared_image_without_sensitive_reads() {
        use std::os::windows::fs::OpenOptionsExt;
        let _apartment = Apartment::enter().unwrap();
        let fixture = MetadataFixture::new();
        let mut guards = Vec::new();
        for file in ["auth.json", "config.toml", "history.jsonl"] {
            std::fs::write(fixture.0.root.join(file), b"SYNTHETIC_NEVER_READ").unwrap();
            guards.push(
                std::fs::OpenOptions::new()
                    .read(true)
                    .share_mode(0)
                    .open(fixture.0.root.join(file))
                    .unwrap(),
            );
        }
        crate::native::take_opened_paths();
        let report = fixture.collect().unwrap();
        assert_eq!(report["images"].as_array().unwrap().len(), 2);
        assert_eq!(report["outer_ancestry_validation"], "not_established");
        assert_eq!(
            report["observation_scope"],
            "partial_registered_root_and_declared_images"
        );
        assert_eq!(report["qualified"], false);
        assert_eq!(report["credential_mutation_enabled"], false);
        assert!(report.get("desktop").is_none());
        assert!(report.get("runtime").is_none());
        for image in report["images"].as_array().unwrap() {
            assert_eq!(
                image["primary_signer"],
                json!({"state": "not_established_offline", "leaf_der_sha256": null})
            );
            assert_eq!(image["current_user_can_replace_or_modify"], "unknown");
            assert_eq!(image["executable_role"], "unqualified_manifest_candidate");
            assert_eq!(image["qualified"], false);
        }
        for field in [
            "publisher_binding",
            "effective_home",
            "effective_backend",
            "effective_policy",
            "runtime_role",
        ] {
            assert_eq!(report[field], "unknown");
        }
        assert!(!encode(&report).unwrap().contains("SYNTHETIC"));
        let opened = crate::native::take_opened_paths();
        assert!(opened.iter().all(|p| p == &fixture.0.root
            || ["AppxManifest.xml", "first.exe", "second.exe"]
                .iter()
                .any(|n| p.file_name().is_some_and(|f| f == *n))));
        drop(guards);
        for file in ["auth.json", "config.toml", "history.jsonl"] {
            assert_eq!(
                std::fs::read(fixture.0.root.join(file)).unwrap(),
                b"SYNTHETIC_NEVER_READ"
            );
        }
    }

    #[test]
    fn cached_signer_report_distinguishes_identity_missing_and_cleanup_without_raw_der() {
        use crate::native::CachedSignature;
        let _apartment = Apartment::enter().unwrap();
        for (input, expected_signature, expected_state) in [
            (
                CachedSignature::NotEstablished,
                "not_established_offline",
                "not_established_offline",
            ),
            (
                CachedSignature::AcceptedSignerUnavailable,
                "cached_trust_accepted_not_official_identity",
                "signer_unavailable",
            ),
            (
                CachedSignature::CleanupFailed,
                "not_established_offline",
                "cleanup_not_established",
            ),
        ] {
            let (signature, report) = signature_report(input).unwrap();
            assert_eq!(signature, expected_signature);
            assert_eq!(
                report,
                json!({"state": expected_state, "leaf_der_sha256": null})
            );
        }
        let (signature, report) =
            signature_report(CachedSignature::AcceptedLeaf(b"abc".to_vec())).unwrap();
        assert_eq!(signature, "cached_trust_accepted_not_official_identity");
        assert_eq!(
            report,
            json!({"state": "observed_not_publisher_approval",
            "leaf_der_sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"})
        );
        assert!(!report.to_string().contains("abc"));
    }

    #[test]
    fn native_registered_metadata_accepts_one_utf8_bom_and_refuses_invalid_prefixes() {
        let _apartment = Apartment::enter().unwrap();
        let fixture = MetadataFixture::new();
        let path = fixture.0.root.join("AppxManifest.xml");
        let manifest = std::fs::read(&path).unwrap();
        let mut prefixed = b"\xef\xbb\xbf".to_vec();
        prefixed.extend_from_slice(&manifest);
        std::fs::write(&path, &prefixed).unwrap();
        let report = fixture.collect().unwrap();
        assert_eq!(report["manifest_identity_matches_registration"], true);
        assert_eq!(report["images"].as_array().unwrap().len(), 2);
        assert_eq!(report["qualified"], false);
        assert_eq!(manifest_executables(&fixture.0.root).unwrap().len(), 2);
        for prefix in [
            b"\xef\xbb\xbf\xef\xbb\xbf".as_slice(),
            b"x\xef\xbb\xbf",
            b"\xef\xbb",
            b"\xff",
        ] {
            let mut invalid = prefix.to_vec();
            invalid.extend_from_slice(&manifest);
            std::fs::write(&path, invalid).unwrap();
            assert!(matches!(fixture.collect(), Err(ReadError::InvalidMetadata)));
        }
    }

    #[test]
    fn native_registered_metadata_refuses_manifest_mismatch_limits_and_external_paths() {
        let _apartment = Apartment::enter().unwrap();
        let fixture = MetadataFixture::new();
        for bytes in [
            b"\xff".to_vec(),
            b"<broken".to_vec(),
            b"<!DOCTYPE Package [<!ENTITY x SYSTEM 'file:///SYNTHETIC'>]><Package>&x;</Package>"
                .to_vec(),
            vec![b' '; MAX_CONFIG_BYTES + 1],
        ] {
            std::fs::write(fixture.0.root.join("AppxManifest.xml"), bytes).unwrap();
            assert!(matches!(
                fixture.collect(),
                Err(ReadError::InvalidMetadata | ReadError::InputLimit)
            ));
        }
        fixture.manifest("<Application Executable='../external.exe'/>");
        assert!(matches!(fixture.collect(), Err(ReadError::UnsafePath)));
        fixture.manifest("<Application Executable='first.exe'/>");
        let mut mismatched = fixture.0.clone();
        mismatched.version[3] = 5;
        assert!(matches!(
            collect_registered(&mismatched, false, || Ok(vec![mismatched.clone()])),
            Err(ReadError::InvalidMetadata)
        ));
        fixture.manifest(&"<Application Executable='first.exe'/>".repeat(MAX_APPLICATIONS + 1));
        assert!(matches!(fixture.collect(), Err(ReadError::InputLimit)));
        fixture.manifest("<Application Executable='first.exe'/>");
        std::fs::write(fixture.0.root.join("first.exe"), b"not a PE").unwrap();
        assert!(matches!(fixture.collect(), Err(ReadError::InvalidMetadata)));
    }

    #[test]
    fn native_registered_metadata_refuses_sharing_hardlinks_and_registration_drift() {
        let _apartment = Apartment::enter().unwrap();
        let fixture = MetadataFixture::new();
        let writer = std::fs::OpenOptions::new()
            .write(true)
            .open(fixture.0.root.join("first.exe"))
            .unwrap();
        assert!(matches!(
            fixture.collect(),
            Err(ReadError::SharingViolation)
        ));
        drop(writer);
        std::fs::hard_link(
            fixture.0.root.join("first.exe"),
            fixture.0.root.join("alias.exe"),
        )
        .unwrap();
        assert!(matches!(fixture.collect(), Err(ReadError::UnsafePath)));
        std::fs::remove_file(fixture.0.root.join("alias.exe")).unwrap();
        for field in 0..5 {
            let mut changed = fixture.0.clone();
            match field {
                0 => changed.publisher.push('x'),
                1 => changed.root.push("other"),
                2 => changed.version[0] += 1,
                3 => changed.origin = "other",
                _ => changed.full_name.push('x'),
            }
            assert!(matches!(
                collect_registered(&fixture.0, false, || Ok(vec![changed])),
                Err(ReadError::Changed)
            ));
        }
        assert!(matches!(
            collect_registered(&fixture.0, false, || Ok(vec![])),
            Err(ReadError::Changed)
        ));
        std::fs::remove_file(fixture.0.root.join("second.exe")).unwrap();
        assert!(matches!(fixture.collect(), Err(ReadError::Missing)));
    }

    #[test]
    fn native_registered_metadata_retains_manifest_root_and_images_through_reobservation() {
        let _apartment = Apartment::enter().unwrap();
        let fixture = MetadataFixture::new();
        let result = collect_registered(&fixture.0, false, || {
            for file in ["AppxManifest.xml", "first.exe", "second.exe"] {
                assert!(std::fs::OpenOptions::new()
                    .write(true)
                    .open(fixture.0.root.join(file))
                    .is_err());
                assert!(std::fs::rename(
                    fixture.0.root.join(file),
                    fixture.0.root.join("substitution.exe")
                )
                .is_err());
            }
            assert!(std::fs::rename(
                &fixture.0.root,
                fixture.0.root.with_extension("substitution")
            )
            .is_err());
            Ok(vec![fixture.0.clone()])
        });
        assert!(result.is_ok());
        let changed = collect_registered(&fixture.0, false, || {
            std::fs::write(fixture.0.root.join("new-synthetic-entry"), b"SYNTHETIC").unwrap();
            Ok(vec![fixture.0.clone()])
        });
        assert!(matches!(changed, Err(ReadError::Changed)));
    }
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
        assert_eq!(
            hash_bytes(b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn native_selected_reads_are_narrow_and_reports_redact_paths() {
        use std::os::windows::fs::OpenOptionsExt;
        let _apartment = Apartment::enter().unwrap();
        let root =
            std::env::temp_dir().join(format!("ca02-SYNTHETIC_CANARY-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let mut pe = vec![0u8; 512];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&128u32.to_le_bytes());
        pe[128..132].copy_from_slice(b"PE\0\0");
        pe[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        std::fs::write(root.join("synthetic.exe"), &pe).unwrap();
        let config =
            b"cli_auth_credentials_store='file'\nforced_chatgpt_workspace_id='SYNTHETIC_CANARY'";
        std::fs::write(root.join("config.toml"), config).unwrap();
        std::fs::write(root.join("auth.json"), b"SYNTHETIC_NEVER_READ").unwrap();
        let auth_guard = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(root.join("auth.json"))
            .unwrap();
        crate::native::take_opened_paths();
        let (binary, image) = binary_observation(&root, "synthetic.exe", false).unwrap();
        let config_observed = config_observation(Some(&root));
        let report = json!({"binary": binary, "config": config_observed.report()});
        let opened = crate::native::take_opened_paths();
        assert!(opened
            .iter()
            .any(|p| p.file_name().is_some_and(|n| n == "synthetic.exe")));
        assert!(opened
            .iter()
            .any(|p| p.file_name().is_some_and(|n| n == "config.toml")));
        assert!(opened.iter().all(|p| p.is_dir()
            || p.file_name()
                .is_some_and(|n| n == "synthetic.exe" || n == "config.toml")));
        let encoded = encode(&report).unwrap();
        assert!(!encoded.contains("SYNTHETIC_CANARY"));
        assert!(!encoded.contains("installed_location"));
        assert_eq!(report["binary"]["qualified"], false);
        assert_eq!(report["config"]["effective_backend"], "unknown");
        image.revalidate().unwrap();
        drop(image);
        drop(auth_guard);
        assert_eq!(std::fs::read(root.join("synthetic.exe")).unwrap(), pe);
        assert_eq!(std::fs::read(root.join("config.toml")).unwrap(), config);
        assert_eq!(
            std::fs::read(root.join("auth.json")).unwrap(),
            b"SYNTHETIC_NEVER_READ"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
