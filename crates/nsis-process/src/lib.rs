#![no_std]
use nsis_plugin_api::*;
nsis_plugin!();

/* start-marker */
extern crate alloc;

use alloc::{borrow::ToOwned, vec};
use core::{mem, ops::Deref, ops::DerefMut, ptr};

use windows_sys::{
    w,
    Win32::{
        Foundation::{
            CloseHandle, GetLastError, ERROR_ELEVATION_REQUIRED, ERROR_INSUFFICIENT_BUFFER, FALSE,
            HANDLE,
        },
        System::Threading::{
            CreateProcessW, InitializeProcThreadAttributeList, OpenProcess,
            UpdateProcThreadAttribute, CREATE_NEW_PROCESS_GROUP, CREATE_UNICODE_ENVIRONMENT,
            EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_CREATE_PROCESS,
            PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, STARTUPINFOEXW,
            STARTUPINFOW,
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

/// Return true if success
///
/// Ported from https://devblogs.microsoft.com/oldnewthing/20190425-00/?p=102443
unsafe fn run_as_user(program: &str, arguments: &str) -> bool {
    let hwnd = GetShellWindow();
    if hwnd.is_null() {
        return false;
    }

    let mut proccess_id = 0;
    if GetWindowThreadProcessId(hwnd, &mut proccess_id) == FALSE as u32 {
        return false;
    }

    let process = OwnedHandle::new(OpenProcess(PROCESS_CREATE_PROCESS, FALSE, proccess_id));
    if process.is_invalid() {
        return false;
    }

    let mut size = 0;
    if !(InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut size) == FALSE
        && GetLastError() == ERROR_INSUFFICIENT_BUFFER)
    {
        return false;
    }

    let mut buffer = vec![0u8; size];
    let attribute_list = buffer.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
    if InitializeProcThreadAttributeList(attribute_list, 1, 0, &mut size) == FALSE {
        return false;
    }

    if UpdateProcThreadAttribute(
        attribute_list,
        0,
        PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as _,
        &*process as *const _ as _,
        mem::size_of::<HANDLE>(),
        ptr::null_mut(),
        ptr::null(),
    ) == FALSE
    {
        return false;
    }

    let startup_info = STARTUPINFOEXW {
        StartupInfo: STARTUPINFOW {
            cb: mem::size_of::<STARTUPINFOEXW>() as _,
            ..mem::zeroed()
        },
        lpAttributeList: attribute_list,
    };
    let mut process_info: PROCESS_INFORMATION = mem::zeroed();
    let mut command_line = "\"".to_owned() + program + "\"";
    if !arguments.is_empty() {
        command_line.push(' ');
        command_line.push_str(arguments);
    }

    let program_wide = encode_utf16(program);

    if CreateProcessW(
        program_wide.as_ptr(),
        encode_utf16(&command_line).as_mut_ptr(),
        ptr::null(),
        ptr::null(),
        FALSE,
        CREATE_UNICODE_ENVIRONMENT | CREATE_NEW_PROCESS_GROUP | EXTENDED_STARTUPINFO_PRESENT,
        ptr::null(),
        ptr::null(),
        &startup_info as *const _ as _,
        &mut process_info,
    ) != FALSE
    {
        CloseHandle(process_info.hProcess);
        CloseHandle(process_info.hThread);
        true
    } else if GetLastError() == ERROR_ELEVATION_REQUIRED {
        let result = ShellExecuteW(
            ptr::null_mut(),
            w!("open"),
            program_wide.as_ptr(),
            encode_utf16(&command_line).as_ptr(),
            ptr::null(),
            SW_SHOW,
        );
        result as isize > 32
    } else {
        false
    }
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
