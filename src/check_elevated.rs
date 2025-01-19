#![deny(missing_docs)]

use std::ffi::c_void;
use std::mem::size_of;
use std::mem::zeroed;

use std::ptr::null_mut;

use windows::Win32::Foundation;
use windows::Win32::Security::GetTokenInformation;
use windows::Win32::Security::TokenElevation;
use windows::Win32::Security::TOKEN_ELEVATION;
use windows::Win32::Security::TOKEN_QUERY;
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::Threading::OpenProcessToken;

/// Returns windows_result::Result<bool>, indicating whether the current process is elevated.
/// ## Example
/// ```rust
/// use windows_elevate::check_elevated;
///
/// fn test_check_elevated() {
///     let is_elevated = check_elevated().expect("Failed to call check_elevated");
///
///     if !is_elevated {
///         print!("You don't have permission to do certain things");
///         return;
///     }
/// }
/// ```
#[cfg(target_os = "windows")]
pub fn check_elevated() -> windows_result::Result<bool> {
    unsafe {
        let h_process = GetCurrentProcess();
        let mut h_token = Foundation::HANDLE(null_mut());
        let open_result = OpenProcessToken(h_process, TOKEN_QUERY, &mut h_token);
        let mut ret_len: u32 = 0;
        let mut token_info: TOKEN_ELEVATION = zeroed();

        if let Err(e) = open_result {
            log::error!("OpenProcessToken {:?}", e);
            return Err(e);
        }

        if let Err(e) = GetTokenInformation(
            h_token,
            TokenElevation,
            Some(std::ptr::addr_of_mut!(token_info).cast::<c_void>()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        ) {
            log::error!("GetTokenInformation {:?}", e);

            return Err(e);
        }

        Ok(token_info.TokenIsElevated != 0)
    }
}
