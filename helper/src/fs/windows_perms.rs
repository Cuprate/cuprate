//! Windows ACL implementation for [`super::set_private_directory_permissions`].

//---------------------------------------------------------------------------------------------------- Use
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use target_os_lib::core::{Error, Owned, Result, PCWSTR, PWSTR};
use target_os_lib::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, ERROR_INSUFFICIENT_BUFFER, E_UNEXPECTED, HANDLE, HLOCAL,
};
use target_os_lib::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use target_os_lib::Win32::Security::{
    GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_ACCESS_MASK,
    TOKEN_QUERY, TOKEN_USER,
};
use target_os_lib::Win32::Storage::FileSystem::CreateDirectoryW;
use target_os_lib::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

//---------------------------------------------------------------------------------------------------- SecurityDescriptor
/// Security descriptor parsed from SDDL, with its `LocalAlloc` buffer freed on drop.
struct SecurityDescriptor {
    psd: PSECURITY_DESCRIPTOR,
    _buf: Owned<HLOCAL>,
}

impl SecurityDescriptor {
    fn from_sddl(sddl: &str) -> Result<Self> {
        let sddl_w = to_wide_nul(OsStr::new(sddl));
        let mut psd = PSECURITY_DESCRIPTOR::default();
        // SAFETY: `sddl_w` is owned and null-terminated; `psd` is a valid out-pointer.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(sddl_w.as_ptr()),
                SDDL_REVISION_1,
                &raw mut psd,
                None,
            )
        }?;
        Ok(Self {
            psd,
            // SAFETY: `psd.0` is a `LocalAlloc` buffer per the function contract.
            _buf: unsafe { Owned::new(HLOCAL(psd.0)) },
        })
    }
}

//---------------------------------------------------------------------------------------------------- Apply
/// Apply a private ACL to each path in `roots`.
///
/// SYSTEM and Administrators are granted access alongside the user so
/// Windows backup, antivirus, and indexer services keep working.
pub(super) fn apply(roots: &[&Path]) {
    let user = match current_user_sid_string() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("warning: could not retrieve user SID: {e}");
            return;
        }
    };
    let sddl = format!("O:{user}D:P(A;OICI;FA;;;{user})(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)");
    let sd = match SecurityDescriptor::from_sddl(&sddl) {
        Ok(sd) => sd,
        Err(e) => {
            eprintln!("warning: could not parse Windows security descriptor: {e}");
            return;
        }
    };

    #[expect(clippy::cast_possible_truncation, reason = "struct size fits in u32")]
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.psd.0,
        bInheritHandle: false.into(),
    };
    for root in roots {
        if let Err(e) = create_private_directory(root, &sa) {
            eprintln!(
                "warning: could not create private directory {}: {e}",
                root.display()
            );
        }
    }
}

//---------------------------------------------------------------------------------------------------- Helpers
fn current_user_sid_string() -> Result<String> {
    let token = open_process_token(TOKEN_QUERY)?;

    let mut len: u32 = 0;
    // SAFETY: probe call with null buffer; `len` is a valid out-pointer.
    match unsafe { GetTokenInformation(*token, TokenUser, None, 0, &raw mut len) } {
        Err(e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => {}
        Err(e) => return Err(e),
        Ok(()) => return Err(Error::from_hresult(E_UNEXPECTED)),
    }

    // `u64` elements satisfy `TOKEN_USER`'s 8-byte alignment.
    let mut buf: Vec<u64> = vec![0; (len as usize).div_ceil(size_of::<u64>())];
    // SAFETY: `buf` is sized per the probe and aligned for `TOKEN_USER`.
    unsafe {
        GetTokenInformation(
            *token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            len,
            &raw mut len,
        )
    }?;

    // SAFETY: `buf` was just populated as a `TOKEN_USER` by the call above.
    let token_user = unsafe { &*buf.as_ptr().cast::<TOKEN_USER>() };
    let mut sid_pwstr = PWSTR::null();
    // SAFETY: `token_user.User.Sid` is valid; out-pointer is owned.
    unsafe { ConvertSidToStringSidW(token_user.User.Sid, &raw mut sid_pwstr) }?;
    // SAFETY: `sid_pwstr.0` is a `LocalAlloc` buffer per `ConvertSidToStringSidW`.
    let _sid_guard = unsafe { Owned::new(HLOCAL(sid_pwstr.0.cast())) };

    // SAFETY: `sid_pwstr` was just populated above. The `to_string` failure
    // case is unreachable for a Windows-emitted SID and maps to `E_UNEXPECTED`.
    unsafe { sid_pwstr.to_string() }.map_err(|_| Error::from_hresult(E_UNEXPECTED))
}

fn create_private_directory(path: &Path, sa: &SECURITY_ATTRIBUTES) -> Result<()> {
    if path.is_dir() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_private_directory(parent, sa)?;
        }
    }

    let path_w = to_wide_nul(path.as_os_str());
    // SAFETY: `path_w` is owned; `sa` outlives this call.
    unsafe { CreateDirectoryW(PCWSTR(path_w.as_ptr()), Some(ptr::from_ref(sa))) }.or_else(|e| {
        if e.code() == ERROR_ALREADY_EXISTS.to_hresult() {
            Ok(())
        } else {
            Err(e)
        }
    })
}

fn open_process_token(access: TOKEN_ACCESS_MASK) -> Result<Owned<HANDLE>> {
    let mut token = HANDLE::default();
    // SAFETY: `token` is a valid out-pointer; the returned handle is owned
    // by the resulting `Owned<HANDLE>`.
    unsafe { OpenProcessToken(GetCurrentProcess(), access, &raw mut token) }?;
    // SAFETY: `OpenProcessToken` returned Ok; `token` is now owned.
    Ok(unsafe { Owned::new(token) })
}

fn to_wide_nul(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}
