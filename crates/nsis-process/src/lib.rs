use std::{ffi::c_void, mem, ptr};

use pluginapi::{decode_wide, exdll_init, popstring, pushint, stack_t, wchar_t};

use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, HWND},
    Security::{EqualSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        Threading::{
            OpenProcess, OpenProcessToken, TerminateProcess, PROCESS_QUERY_INFORMATION,
            PROCESS_TERMINATE,
        },
    },
};

/// Test if there is a running process with the given name, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// # Safety
///
/// This function always expects 1 string on the stack ($1: name) and will panic otherwise.
#[no_mangle]
pub unsafe extern "C" fn FindProcess(
    _hwnd_parent: HWND,
    string_size: u32,
    variables: *mut wchar_t,
    stacktop: *mut *mut stack_t,
) {
    exdll_init(string_size, variables, stacktop);

    let name = popstring().unwrap();

    if !get_processes(&name).is_empty() {
        pushint(0);
    } else {
        pushint(1);
    }
}

/// Test if there is a running process with the given name that belongs to the current user, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// # Safety
///
/// This function always expects 1 string on the stack ($1: name) and will panic otherwise.
#[no_mangle]
pub unsafe extern "C" fn FindProcessCurrentUser(
    _hwnd_parent: HWND,
    string_size: u32,
    variables: *mut wchar_t,
    stacktop: *mut *mut stack_t,
) {
    exdll_init(string_size, variables, stacktop);

    let name = popstring().unwrap();

    let processes = get_processes(&name);

    if let Some(user_sid) = get_sid(std::process::id()) {
        if processes
            .into_iter()
            .any(|pid| belongs_to_user(user_sid, pid))
        {
            pushint(0);
        } else {
            pushint(1);
        }
    // Fall back to perMachine checks if we can't get current user id
    } else if processes.is_empty() {
        pushint(1);
    } else {
        pushint(0);
    }
}

/// Kill all running process with the given name, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// # Safety
///
/// This function always expects 1 string on the stack ($1: name) and will panic otherwise.
#[no_mangle]
pub unsafe extern "C" fn KillProcess(
    _hwnd_parent: HWND,
    string_size: u32,
    variables: *mut wchar_t,
    stacktop: *mut *mut stack_t,
) {
    exdll_init(string_size, variables, stacktop);

    let name = popstring().unwrap();

    let processes = get_processes(&name);

    if !processes.is_empty() && processes.into_iter().map(kill).all(|b| b) {
        pushint(0);
    } else {
        pushint(1);
    }
}

/// Kill all running process with the given name that belong to the current user, skipping processes with the host's pid. The input and process names are case-insensitive.
///
/// # Safety
///
/// This function always expects 1 string on the stack ($1: name) and will panic otherwise.
#[no_mangle]
pub unsafe extern "C" fn KillProcessCurrentUser(
    _hwnd_parent: HWND,
    string_size: u32,
    variables: *mut wchar_t,
    stacktop: *mut *mut stack_t,
) {
    exdll_init(string_size, variables, stacktop);

    let name = popstring().unwrap();

    let processes = get_processes(&name);

    if processes.is_empty() {
        pushint(1);
        return;
    }

    let success = if let Some(user_sid) = get_sid(std::process::id()) {
        processes
            .into_iter()
            .filter(|pid| belongs_to_user(user_sid, *pid))
            .map(kill)
            .all(|b| b)
    } else {
        processes.into_iter().map(kill).all(|b| b)
    };

    if success {
        pushint(0)
    } else {
        pushint(1)
    }
}

unsafe fn belongs_to_user(user_sid: *mut c_void, pid: u32) -> bool {
    let p_sid = get_sid(pid);
    // Trying to get the sid of a process of another user will give us an "Access Denied" error.
    // TODO: Consider checking for HRESULT(0x80070005) if we want to return true for other errors to try and kill those processes later.
    p_sid
        .map(|p_sid| EqualSid(user_sid, p_sid) != 0)
        .unwrap_or_default()
}

fn kill(pid: u32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        let success = TerminateProcess(handle, 1);
        CloseHandle(handle);
        success != 0
    }
}

// Get the SID of a process. Returns None on error.
unsafe fn get_sid(pid: u32) -> Option<*mut c_void> {
    let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);

    let mut sid = None;
    let mut token_handle = HANDLE::default();

    if OpenProcessToken(handle, TOKEN_QUERY, &mut token_handle) != 0 {
        let mut info_length = 0;

        GetTokenInformation(
            token_handle,
            TokenUser,
            ptr::null_mut(),
            0,
            &mut info_length as *mut u32,
        );

        // GetTokenInformation always returns 0 for the first call so we check if it still gave us the buffer length
        if info_length == 0 {
            return sid;
        }

        let info = vec![0u8; info_length as usize].as_mut_ptr() as *mut TOKEN_USER;

        if GetTokenInformation(
            token_handle,
            TokenUser,
            info as *mut c_void,
            info_length,
            &mut info_length,
        ) == 0
        {
            return sid;
        }

        sid = Some((*info).User.Sid)
    }

    CloseHandle(token_handle);
    CloseHandle(handle);

    sid
}

fn get_processes(name: &str) -> Vec<u32> {
    let current_pid = std::process::id();
    let mut processes = Vec::new();

    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);

        let mut process = PROCESSENTRY32W {
            dwSize: mem::size_of::<PROCESSENTRY32W>() as u32,
            ..mem::zeroed()
        };

        if Process32FirstW(handle, &mut process) != 0 {
            while Process32NextW(handle, &mut process) != 0 {
                if current_pid != process.th32ProcessID
                    && decode_wide(&process.szExeFile)
                        .to_str()
                        .unwrap_or_default()
                        .to_lowercase()
                        == name.to_lowercase()
                {
                    processes.push(process.th32ProcessID);
                }
            }
        }

        CloseHandle(handle);
    }

    processes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_process() {
        let processes = get_processes("explorer.exe");
        dbg!(&processes);
        assert!(!processes.is_empty());
    }

    #[test]
    fn kill_process() {
        let processes = get_processes("something_that_doesnt_exist.exe");
        dbg!(&processes);
        // TODO: maybe find some way to spawn a dummy process we can kill here?
        // This will return true on empty iterators so it's basically no-op right now
        assert!(processes.into_iter().map(kill).all(|b| b));
    }
}
