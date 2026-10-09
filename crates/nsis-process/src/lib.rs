#![no_std]
use nsis_plugin_api::*;
nsis_plugin!();

/* start-marker */
extern crate alloc;

use alloc::borrow::ToOwned;
use core::{
    ffi::c_void,
    mem,
    ops::{Deref, DerefMut},
    ptr,
};

use windows_sys::{
    core::PCWSTR,
    w,
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_ELEVATION_REQUIRED, ERROR_NOT_ALL_ASSIGNED, FALSE,
            HANDLE, LUID,
        },
        Security::{
            AdjustTokenPrivileges, DuplicateTokenEx, GetTokenInformation, LookupPrivilegeValueW,
            SecurityAnonymous, TokenElevation, TokenPrimary, LUID_AND_ATTRIBUTES,
            SE_IMPERSONATE_NAME, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_DEFAULT,
            TOKEN_ADJUST_PRIVILEGES, TOKEN_ADJUST_SESSIONID, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE,
            TOKEN_ELEVATION, TOKEN_PRIVILEGES, TOKEN_QUERY,
        },
        System::Threading::{
            CreateProcessWithTokenW, GetCurrentProcess, OpenProcess, OpenProcessToken,
            PROCESS_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, STARTUPINFOW,
        },
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId, SW_SHOW},
        },
    },
};

/// Run program as unelevated user
///
/// This function takes 2 strings on the stack as parameters:
///
/// - $1: program
/// - $2: arguments
#[nsis_fn]
fn RunAsUser() -> Result<(), Error> {
    let program = popstr()?;
    let arguments = popstr()?;
    if run_as_user(&program, &arguments) {
        push(ZERO)
    } else {
        push(ONE)
    }
}

unsafe fn is_elevated(process: HANDLE) -> bool {
    let mut token_handle: HANDLE = ptr::null_mut();

    if OpenProcessToken(process, TOKEN_QUERY, &mut token_handle) == FALSE {
        return false;
    }

    let _token = OwnedHandle::new(token_handle);

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut size: u32 = 0;

    let result = GetTokenInformation(
        token_handle,
        TokenElevation,
        &mut elevation as *mut _ as *mut _,
        mem::size_of::<TOKEN_ELEVATION>() as u32,
        &mut size,
    );
    result != FALSE && elevation.TokenIsElevated != 0
}

unsafe fn set_privilege(process: HANDLE, privilege: PCWSTR, enable: bool) -> Option<bool> {
    let mut token: HANDLE = ptr::null_mut();
    if OpenProcessToken(process, TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES, &mut token) == FALSE {
        return None;
    }
    let token = OwnedHandle::new(token);

    let mut luid = LUID::default();
    if LookupPrivilegeValueW(ptr::null(), privilege, &mut luid) == FALSE {
        return None;
    }

    let token_privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: if enable { SE_PRIVILEGE_ENABLED } else { 0 },
        }],
    };

    let mut previous_state = TOKEN_PRIVILEGES::default();
    let mut return_length = 0;
    let result = AdjustTokenPrivileges(
        *token,
        FALSE,
        &token_privileges,
        mem::size_of::<TOKEN_PRIVILEGES>() as u32,
        &mut previous_state,
        &mut return_length,
    );
    if result == FALSE || GetLastError() == ERROR_NOT_ALL_ASSIGNED {
        return None;
    }

    if previous_state.PrivilegeCount == 1 {
        let was_enabled = (previous_state.Privileges[0].Attributes & SE_PRIVILEGE_ENABLED) != 0;
        Some(was_enabled)
    } else {
        Some(enable)
    }
}

/// Return true if success
///
/// Ported from https://source.chromium.org/chromium/chromium/src/+/main:base/win/elevation_util.cc;drc=36e1c43ace542988d624bd1bc0813c184482d2ab;l=69
/// Based on https://learn.microsoft.com/en-us/archive/blogs/aaron_margosis/faq-how-do-i-start-a-program-as-the-desktop-user-from-an-elevated-app
unsafe fn run_as_user(program: &str, arguments: &str) -> bool {
    let current_process = GetCurrentProcess();
    if !is_elevated(current_process) {
        // Launch directly if no admin access
        return shell_execute(&encode_utf16(program), arguments);
    }

    let hwnd = GetShellWindow();
    if hwnd.is_null() {
        return false;
    }

    let mut process_id = 0;
    if GetWindowThreadProcessId(hwnd, &mut process_id) == FALSE as u32 {
        return false;
    }

    let process = OwnedHandle::new(OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION,
        FALSE,
        process_id,
    ));
    if process.is_invalid() {
        return false;
    }

    let privilege = SE_IMPERSONATE_NAME;
    let Some(enabled_previously) = set_privilege(current_process, privilege, true) else {
        return false;
    };
    let _impersonate_guard = RevertPrivilegeOnDrop {
        process: current_process,
        privilege,
        previous_state: enabled_previously,
    };

    let mut handle_token: HANDLE = ptr::null_mut();
    if OpenProcessToken(*process, TOKEN_QUERY | TOKEN_DUPLICATE, &mut handle_token) == FALSE {
        return false;
    }
    let handle_token = OwnedHandle::new(handle_token);

    let mut handle_new_token: HANDLE = ptr::null_mut();
    if DuplicateTokenEx(
        *handle_token,
        TOKEN_QUERY
            | TOKEN_ASSIGN_PRIMARY
            | TOKEN_DUPLICATE
            | TOKEN_ADJUST_DEFAULT
            | TOKEN_ADJUST_SESSIONID,
        ptr::null(),
        SecurityAnonymous,
        TokenPrimary,
        &mut handle_new_token,
    ) == FALSE
    {
        return false;
    }
    let handle_new_token = OwnedHandle::new(handle_new_token);

    let program_wide = encode_utf16(program);
    let mut command_line = "\"".to_owned() + program + "\"";
    if !arguments.is_empty() {
        command_line.push(' ');
        command_line.push_str(arguments);
    }
    let mut command_line_wide = encode_utf16(&command_line);

    let startup_info = STARTUPINFOW {
        cb: mem::size_of::<STARTUPINFOW>() as u32,
        ..mem::zeroed()
    };
    let mut process_info: PROCESS_INFORMATION = mem::zeroed();

    let success = CreateProcessWithTokenW(
        *handle_new_token,
        0,
        program_wide.as_ptr(),
        command_line_wide.as_mut_ptr(),
        0,
        ptr::null(),
        ptr::null(),
        &startup_info,
        &mut process_info,
    );

    if success != FALSE {
        CloseHandle(process_info.hProcess);
        CloseHandle(process_info.hThread);
        true
    } else if GetLastError() == ERROR_ELEVATION_REQUIRED {
        shell_execute(&program_wide, arguments)
    } else {
        false
    }
}

fn shell_execute(program_wide: &[u16], arguments: &str) -> bool {
    let arguments_wide = encode_utf16(arguments);
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            w!("open"),
            program_wide.as_ptr(),
            arguments_wide.as_ptr(),
            ptr::null(),
            SW_SHOW,
        )
    };
    result as isize > 32
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(handle: HANDLE) -> Self {
        Self(handle)
    }

    fn is_invalid(&self) -> bool {
        self.0.is_null()
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.is_invalid() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

impl Deref for OwnedHandle {
    type Target = HANDLE;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OwnedHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

struct RevertPrivilegeOnDrop {
    process: *mut c_void,
    privilege: *const u16,
    previous_state: bool,
}

impl Drop for RevertPrivilegeOnDrop {
    fn drop(&mut self) {
        unsafe {
            set_privilege(self.process, self.privilege, self.previous_state);
        };
    }
}

/* end-marker */

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn spawn_cmd() {
        unsafe { run_as_user("cmd", "/c timeout 3") };
    }

    #[test]
    #[cfg(feature = "test")]
    fn spawn_with_spaces() {
        extern crate std;
        use alloc::format;
        use alloc::string::ToString;

        let current = std::env::current_dir().unwrap();

        let dir = current.join("dir space");
        std::fs::create_dir_all(&dir).unwrap();

        let systemroot = std::env::var("SYSTEMROOT").unwrap_or_else(|_| "C:\\Windows".to_owned());

        let cmd = format!("{systemroot}\\System32\\cmd.exe");
        let cmd_out = dir.join("cmdout.exe");

        std::fs::copy(cmd, &cmd_out).unwrap();

        assert!(unsafe { run_as_user(cmd_out.display().to_string().as_str(), "/c timeout 3") });

        std::thread::sleep(std::time::Duration::from_secs(5));
        std::fs::remove_file(cmd_out).unwrap();
    }
}
