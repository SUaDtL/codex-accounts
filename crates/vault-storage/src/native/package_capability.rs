//! Fixed official package capability identity. The package family is already
//! fixed by installation discovery; this is independent of Desktop versions.
//! Windows derives the package SID from OpenAI.Codex_2p2nqsd0c76g0. Its package
//! capability uses RID 3 instead of package RID 2, retaining the seven hash RIDs.
//! Never admit arbitrary capability families or use this for credential access.
use super::*;

// Public, version-independent identity derived with Windows
// DeriveAppContainerSidFromAppContainerName for the fixed official package family,
// then represented as its package capability (RID 3). This is not a user/account
// SID or the unrelated Codex cap_sid ledger. No broad capability match is used.
const CODEX_PACKAGE_CAPABILITY: &str =
    "S-1-15-3-2968813833-811790644-2202111208-3784096404-1081847329-2708967783-1438471679";

pub(super) fn resolve() -> Option<Vec<u8>> {
    let name: Vec<u16> = CODEX_PACKAGE_CAPABILITY
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut sid = null_mut();
    // SAFETY: fixed public SID literal and LocalFree-owned Windows output.
    if unsafe { ConvertStringSidToSidW(name.as_ptr(), &mut sid) } == 0 {
        return None;
    }
    let sid = Allocation(sid);
    sid_copy(sid.0).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_package_grant_is_accepted_only_for_ancestors() {
        let fixture = super::super::tests::Sandbox::new();
        let security = Security::new().unwrap();
        let name = wide(&fixture.0).unwrap();
        assert_ne!(
            unsafe { CreateDirectoryW(name.as_ptr(), &security.attrs()) },
            0
        );
        let file = open_file(
            &fixture.0,
            READ_CONTROL | WRITE_DAC | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            OPEN_EXISTING,
            None,
        )
        .unwrap();
        let mut text = null_mut();
        assert_ne!(
            unsafe { ConvertSidToStringSidW(security.user.as_ptr().cast_mut().cast(), &mut text) },
            0
        );
        let text = Allocation(text.cast());
        let mut length = 0;
        while length < 256 && unsafe { *text.0.cast::<u16>().add(length) } != 0 {
            length += 1;
        }
        assert!(length < 256);
        let user =
            String::from_utf16(unsafe { std::slice::from_raw_parts(text.0.cast::<u16>(), length) })
                .unwrap();
        let apply = |principal: &str| {
            let sddl: Vec<u16> =
                format!("O:{user}D:P(A;;FA;;;{user})(A;;FA;;;SY)(A;;FA;;;{principal})\0")
                    .encode_utf16()
                    .collect();
            let mut descriptor = null_mut();
            assert_ne!(
                unsafe {
                    ConvertStringSecurityDescriptorToSecurityDescriptorW(
                        sddl.as_ptr(),
                        SDDL_REVISION_1,
                        &mut descriptor,
                        null_mut(),
                    )
                },
                0
            );
            let descriptor = Allocation(descriptor);
            let mut acl = null_mut();
            let mut present = 0;
            let mut defaulted = 0;
            assert_ne!(
                unsafe {
                    GetSecurityDescriptorDacl(descriptor.0, &mut present, &mut acl, &mut defaulted)
                },
                0
            );
            assert_eq!(
                unsafe {
                    SetSecurityInfo(
                        file.as_raw_handle(),
                        SE_FILE_OBJECT,
                        DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                        null_mut(),
                        null_mut(),
                        acl,
                        null_mut(),
                    )
                },
                0
            );
        };
        apply(CODEX_PACKAGE_CAPABILITY);
        assert_eq!(security.check(&file, false), Ok(()));
        assert_eq!(security.check(&file, true), Err(StorageError::UnsafePath));
        assert_eq!(
            security.check_credential(&file),
            Err(StorageError::UnsafePath)
        );
        apply("S-1-15-3-1-2-3-4-5-6-7");
        assert_eq!(security.check(&file, false), Err(StorageError::UnsafePath));
    }
}
