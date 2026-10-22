//! Existing official sandbox read access is compatible with active auth files.
//! This never creates a principal or changes a descriptor, and never applies to
//! the encrypted vault, staging-file creation, ownership, or ancestor checks.
use super::*;

pub(super) fn resolve() -> Option<Vec<u8>> {
    let mut computer = [0_u16; 256];
    let mut computer_len = computer.len() as u32;
    // Fixed local-machine query; never trust an environment-supplied domain/name.
    if unsafe {
        windows_sys::Win32::System::WindowsProgramming::GetComputerNameW(
            computer.as_mut_ptr(),
            &mut computer_len,
        )
    } == 0
        || computer_len == 0
        || computer_len as usize >= computer.len()
    {
        return None;
    }
    let computer = &computer[..computer_len as usize];
    let mut account = computer.to_vec();
    account.extend("\\CodexSandboxUsers\0".encode_utf16());
    let mut sid = [0_u64; 16];
    let mut sid_len = std::mem::size_of_val(&sid) as u32;
    let mut domain = [0_u16; 256];
    let mut domain_len = domain.len() as u32;
    let mut kind = 0;
    // Local machine-qualified lookup into fixed aligned buffers. Missing group,
    // domain lookup, wrong principal type, and oversized output grant no exception.
    if unsafe {
        LookupAccountNameW(
            null(),
            account.as_ptr(),
            sid.as_mut_ptr().cast(),
            &mut sid_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut kind,
        )
    } == 0
        || kind != SidTypeAlias
        || sid_len as usize > std::mem::size_of_val(&sid)
        || domain_len as usize >= domain.len()
    {
        return None;
    }
    let domain_end = domain.iter().position(|unit| *unit == 0)?;
    let actual_domain = String::from_utf16(&domain[..domain_end]).ok()?;
    let expected_domain = String::from_utf16(computer).ok()?;
    if !actual_domain.eq_ignore_ascii_case(&expected_domain) {
        return None;
    }
    let copied = sid_copy(sid.as_mut_ptr().cast()).ok()?;
    // Local SAM alias: S-1-5-21-machine-machine-machine-RID. Never accept a
    // well-known/world SID or another SID family as the named sandbox group.
    if copied.len() != 28
        || copied[..8] != [1, 5, 0, 0, 0, 0, 0, 5]
        || copied[8..12] != 21_u32.to_le_bytes()
    {
        return None;
    }
    Some(copied)
}

pub(super) fn permits(group: Option<&[u8]>, principal: &[u8], mask: u32) -> bool {
    let read_only = FILE_GENERIC_READ | FILE_GENERIC_EXECUTE | GENERIC_READ | GENERIC_EXECUTE;
    group == Some(principal) && mask & !read_only == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exception_requires_resolved_group_and_only_read_rights() {
        let group = [1, 2, 3];
        assert!(permits(
            Some(&group),
            &group,
            FILE_GENERIC_READ | FILE_GENERIC_EXECUTE
        ));
        assert!(!permits(None, &group, FILE_GENERIC_READ));
        assert!(!permits(Some(&group), &[4, 5, 6], FILE_GENERIC_READ));
        for right in [
            FILE_WRITE_DATA,
            FILE_APPEND_DATA,
            FILE_WRITE_EA,
            FILE_WRITE_ATTRIBUTES,
            FILE_DELETE_CHILD,
            DELETE,
            WRITE_DAC,
            WRITE_OWNER,
            GENERIC_WRITE,
            GENERIC_ALL,
            0x0200_0000, // MAXIMUM_ALLOWED is not a read-only ACE grant.
        ] {
            assert!(!permits(Some(&group), &group, FILE_GENERIC_READ | right));
        }
    }
}
