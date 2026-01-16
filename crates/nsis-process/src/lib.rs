#![no_std]
use nsis_plugin_api::*;
nsis_plugin!();

/* start-marker */
extern crate alloc;

use alloc::{borrow::ToOwned, vec, vec::Vec};
use core::{ffi::c_void, mem, ops::Deref, ops::DerefMut, ptr};

use windows_sys::{
    core::PCWSTR,
    w,
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_ELEVATION_REQUIRED,
            ERROR_INVALID_PARAMETER, ERROR_NOT_ALL_ASSIGNED, FALSE, HANDLE, LUID, TRUE,
        },
        Security::{
            AdjustTokenPrivileges, DuplicateTokenEx, EqualSid, GetTokenInformation,
            LookupPrivilegeValueW, SecurityAnonymous, TokenElevation, TokenPrimary, TokenUser,
            LUID_AND_ATTRIBUTES, SE_IMPERSONATE_NAME, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_DEFAULT,
            TOKEN_ADJUST_PRIVILEGES, TOKEN_ADJUST_SESSIONID, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE,
            TOKEN_ELEVATION, TOKEN_PRIVILEGES, TOKEN_QUERY, TOKEN_USER,
        },
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                CreateProcessWithTokenW, GetCurrentProcess, GetCurrentProcessId, OpenProcess,
                OpenProcessToken, TerminateProcess, PROCESS_INFORMATION, PROCESS_QUERY_INFORMATION,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, STARTUPINFOW,
            },
        },
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId, SW_SHOW},
        },
    },
};

/// Test if there is a running process with the given name, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// This function takes 1 string on the stack as the parameter:
///
/// - $1: name
#[nsis_fn]
fn FindProcess() -> Result<(), Error> {
    let name = popstr()?;

    if !get_processes(&name).is_empty() {
        push(ZERO)
    } else {
        push(ONE)
    }
}

/// Test if there is a running process with the given name that belongs to the current user, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// This function takes 1 string on the stack as the parameter:
///
/// - $1: name
#[nsis_fn]
fn FindProcessCurrentUser() -> Result<(), Error> {
    let name = popstr()?;

    let processes = get_processes(&name);

    if let Some(user_sid) = get_sid(GetCurrentProcessId()) {
        if processes
            .into_iter()
            .any(|pid| belongs_to_user(user_sid, pid))
        {
            push(ZERO)
        } else {
            push(ONE)
        }
    // Fall back to perMachine checks if we can't get current user id
    } else if processes.is_empty() {
        push(ONE)
    } else {
        push(ZERO)
    }
}

/// Kill all running process with the given name, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// Returns:
///
/// - 0: When one or more processes matching the name were found and killed successfully
/// - 1: When one or more processes matching the name were found but not all were killed successfully
/// - 2: When no processes matching the name were found
///
/// This function takes 1 string on the stack as the parameter:
///
/// - $1: name
#[nsis_fn]
fn KillProcess() -> Result<(), Error> {
    let name = popstr()?;

    let processes = get_processes(&name);

    if processes.is_empty() {
        return push(TWO);
    }

    if processes.into_iter().all(kill) {
        push(ZERO)
    } else {
        push(ONE)
    }
}

/// Kill all running process with the given name that belong to the current user, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// Returns:
///
/// - 0: When one or more processes matching the name were found and killed successfully
/// - 1: When one or more processes matching the name were found but not all were killed successfully
/// - 2: When no processes matching the name were found
///
/// This function takes 1 string on the stack as the parameter:
///
/// - $1: name
#[nsis_fn]
fn KillProcessCurrentUser() -> Result<(), Error> {
    let name = popstr()?;

    let processes = get_processes(&name);

    if processes.is_empty() {
        return push(TWO);
    }

    let success = if let Some(user_sid) = get_sid(GetCurrentProcessId()) {
        processes
            .into_iter()
            .filter(|pid| belongs_to_user(user_sid, *pid))
            .all(kill)
    } else {
        processes.into_iter().all(kill)
    };

    if success {
        push(ZERO)
    } else {
        push(ONE)
    }
}

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

unsafe fn belongs_to_user(user_sid: *mut c_void, pid: u32) -> bool {
    let p_sid = get_sid(pid);
    // Trying to get the sid of a process of another user will give us an "Access Denied" error.
    // TODO: Consider checking for HRESULT(0x80070005) if we want to return true for other errors to try and kill those processes later.
    p_sid
        .map(|p_sid| EqualSid(user_sid, p_sid) != FALSE)
        .unwrap_or_default()
}

fn kill(pid: u32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            let error = GetLastError();
            // ERROR_INVALID_PARAMETER will occur if the process is already terminated
            return error == ERROR_INVALID_PARAMETER;
        }

        let handle = OwnedHandle::new(handle);
        if TerminateProcess(*handle, 1) == FALSE {
            let error = GetLastError();
            // ERROR_ACCESS_DENIED will occur if the process is terminated
            // between OpenProcess and TerminateProcess.
            // If current process lacks permission to terminate process,
            // `OpenProcess` would fail with ERROR_ACCESS_DENIED instead.
            return error == ERROR_ACCESS_DENIED;
        }
        true
    }
}

// Get the SID of a process. Returns None on error.
unsafe fn get_sid(pid: u32) -> Option<*mut c_void> {
    let handle = OwnedHandle::new(OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid));
    if handle.is_invalid() {
        return None;
    }

    let mut token_handle = OwnedHandle::new(ptr::null_mut());
    if OpenProcessToken(*handle, TOKEN_QUERY, &mut *token_handle) == FALSE {
        return None;
    }

    let mut info_length = 0;
    GetTokenInformation(
        *token_handle,
        TokenUser,
        ptr::null_mut(),
        0,
        &mut info_length,
    );
    // GetTokenInformation always returns 0 for the first call so we check if it still gave us the buffer length
    if info_length == 0 {
        return None;
    }

    let mut buffer = vec![0u8; info_length as usize];
    let info = buffer.as_mut_ptr() as *mut TOKEN_USER;
    if GetTokenInformation(
        *token_handle,
        TokenUser,
        info as *mut c_void,
        info_length,
        &mut info_length,
    ) == FALSE
    {
        None
    } else {
        Some((*info).User.Sid)
    }
}

fn get_processes(name: &str) -> Vec<u32> {
    let current_pid = unsafe { GetCurrentProcessId() };
    let mut processes = Vec::new();

    unsafe {
        let handle = OwnedHandle::new(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0));

        let mut process = PROCESSENTRY32W {
            dwSize: mem::size_of::<PROCESSENTRY32W>() as u32,
            ..mem::zeroed()
        };

        if Process32FirstW(*handle, &mut process) == TRUE {
            while Process32NextW(*handle, &mut process) == TRUE {
                if current_pid != process.th32ProcessID
                    && decode_utf16_lossy(&process.szExeFile).to_lowercase() == name.to_lowercase()
                {
                    processes.push(process.th32ProcessID);
                }
            }
        }
    }

    processes
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
    fn find_process() {
        let processes = get_processes("explorer.exe");
        assert!(!processes.is_empty());
    }

    #[test]
    fn kill_process() {
        let processes = get_processes("something_that_doesnt_exist.exe");
        // TODO: maybe find some way to spawn a dummy process we can kill here?
        // This will return true on empty iterators so it's basically no-op right now
        assert!(processes.into_iter().all(kill));
    }

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
