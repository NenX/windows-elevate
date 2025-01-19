#![deny(missing_docs)]

use std::env;
use std::env::args;
use std::ffi::OsStr;
use std::fs;
use std::mem::size_of;

use windows::core::w;
use windows::core::Error;
use windows::core::HSTRING;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};

use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

fn elevate_sh_exec(cmd: &OsStr, arguments: &OsStr) -> windows_result::Result<()> {
    let s_exe = cmd.to_str().unwrap();

    let cwd_path = env::current_dir().unwrap();
    let cwd = cwd_path.to_str().unwrap().to_string();
    let h_cwd = HSTRING::from(&cwd);

    let mixed_args = format!("{} {}", arguments.to_str().unwrap(), "");

    let cmd_file = cwd_path.join("tmp.cmd");
    let exe_file = cwd_path.join(s_exe);
    let stdout_file = cwd_path.join("stdout_file");
    let stderr_file = cwd_path.join("stderr_file");

    let content = format!(
        "@echo off\r\n chcp 65001>nul \r\n call {} {} > {} 2> {}",
        exe_file.to_str().unwrap(),
        mixed_args,
        stdout_file.to_str().unwrap(),
        stderr_file.to_str().unwrap()
    );
    let h_cmd_param = HSTRING::from(format!("/C {}", cmd_file.to_str().unwrap()));

    fs::write(&cmd_file, &content).expect("write cmd file");

    let f_mask = SEE_MASK_NOCLOSEPROCESS;

    let mut sh_exec_info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: f_mask,
        lpVerb: w!("runas"),
        // lpFile: PCWSTR(h_exe.as_ptr()),
        lpFile: w!("cmd.exe"),
        lpDirectory: PCWSTR(h_cwd.as_ptr()),

        lpParameters: PCWSTR(h_cmd_param.as_ptr()),

        nShow: SW_HIDE.0 as i32,
        ..SHELLEXECUTEINFOW::default()
    };

    if let Err(e) = unsafe { ShellExecuteExW(&mut sh_exec_info) } {
        log::error!("ShellExecuteExW: {e:?}");

        return Err(e);
    }

    let r = unsafe { WaitForSingleObject(sh_exec_info.hProcess, INFINITE) };
    if r.0 != WAIT_OBJECT_0.0 {
        let e = Error::from_win32();
        log::error!("WaitForSingleObject: {:?} {:?}", e, cmd);

        return Err(e);
    }
    let stdout = fs::read_to_string(&stdout_file).expect("Failed to read stdout_file");
    let stderr = fs::read_to_string(&stderr_file).expect("Failed to read stderr_file");
    println!(
        "{}{}{}",
        stdout,
        if stderr.len() > 1 { "\n" } else { "" },
        stderr
    );

    fs::remove_file(&stdout_file).expect("Failed to remove stdout_file");
    fs::remove_file(&stderr_file).expect("Failed to remove stderr_file");
    fs::remove_file(&cmd_file).expect("Failed to remove cmd_file");

    let mut status = 0u32;
    if let Err(e) = unsafe { GetExitCodeProcess(sh_exec_info.hProcess, &mut status) } {
        log::error!("GetExitCodeProcess: {e:?}");
        return Err(e);
    }
    unsafe {
        let a = CloseHandle(sh_exec_info.hProcess);
        match a {
            Ok(_) => Ok(()),
            Err(e) => {
                log::error!("CloseHandle Bad: {:?}", e);
                Err(e)
            }
        }
    }
}

/// This function spawns a new elevated process for the current executable file
/// ## Example
/// ```rust
/// use windows_elevate::{check_elevated, elevate};
///
/// fn test_elevate() {
///     let is_elevated = check_elevated().expect("Failed to call check_elevated");
///
///     if !is_elevated {
///         elevate().expect("Failed to elevate");
///         return;
///     }
///     // From here, it's the new elevated process
/// }
/// ```
#[cfg(target_os = "windows")]
pub fn elevate() -> windows_result::Result<()> {
    let mut arguments = args();
    let exe_name = arguments.next().unwrap().clone();
    let exe_path = std::path::Path::new(&exe_name);
    let file_name = exe_path.file_name().unwrap().to_str().unwrap();

    let args_v: Vec<String> = arguments.collect();

    let ja = args_v
        .iter()
        .cloned()
        .reduce(|a, b| a + " " + &b)
        .unwrap_or("".to_string());

    elevate_sh_exec(file_name.as_ref(), ja.as_ref())
}
