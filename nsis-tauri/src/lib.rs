#![allow(clippy::missing_safety_doc)]

use nsis_download::download_file;
use nsis_process::{get_processes, kill};
use nsis_semvercompare::semver_compare;
use nsis_utils::{exdll_init, popstring, pushint, stack_t, wchar_t};
use windows_sys::Win32::Foundation::HWND;

#[no_mangle]
pub unsafe extern "C" fn Download(
    _hwnd_parent: HWND,
    string_size: u32,
    variables: *mut wchar_t,
    stacktop: *mut *mut stack_t,
) {
    exdll_init(string_size, variables, stacktop);

    let url = popstring().unwrap();
    let path = popstring().unwrap();

    let status = download_file(&url, &path);
    pushint(status);
}

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

#[no_mangle]
pub unsafe extern "C" fn SemverCompare(
    _hwnd_parent: HWND,
    string_size: u32,
    variables: *mut wchar_t,
    stacktop: *mut *mut stack_t,
) {
    exdll_init(string_size, variables, stacktop);

    let v1 = popstring().unwrap();
    let v2 = popstring().unwrap();

    let ret = semver_compare(&v1, &v2);
    pushint(ret);
}
