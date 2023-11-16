#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]

use std::{
    ffi::{OsStr, OsString},
    iter::once,
    mem::{size_of, size_of_val},
    os::windows::prelude::{OsStrExt, OsStringExt},
};

use windows_sys::Win32::{
    Foundation::GlobalFree,
    Globalization::{lstrcpyW, lstrcpynW},
    System::Memory::{GlobalAlloc, GPTR},
};

static mut G_STRINGSIZE: u32 = 0;
static mut G_VARIABLES: *mut wchar_t = std::ptr::null_mut();
static mut G_STACKTOP: *mut *mut stack_t = std::ptr::null_mut();

pub unsafe fn exdll_init(string_size: u32, variables: *mut wchar_t, stacktop: *mut *mut stack_t) {
    G_STRINGSIZE = string_size;
    G_VARIABLES = variables;
    G_STACKTOP = stacktop;
}

pub type wchar_t = i32;

#[derive(Debug)]
pub enum Error {
    InvalidStackError,
    InvalidUnicode,
}

#[repr(C)]
#[derive(Debug)]
pub struct stack_t {
    next: *mut stack_t,
    text: [wchar_t; 1],
}

pub unsafe fn pushstring(s: impl AsRef<OsStr>) {
    if G_STACKTOP.is_null() {
        return;
    }

    let string_wide = encode_wide(s);
    let th: *mut stack_t = GlobalAlloc(
        GPTR,
        size_of::<stack_t>() + G_STRINGSIZE as usize * size_of_val(&string_wide),
    ) as _;
    lstrcpynW(
        (*th).text.as_ptr() as _,
        string_wide.as_ptr() as _,
        G_STRINGSIZE as _,
    );
    (*th).next = *G_STACKTOP;
    *G_STACKTOP = th;
}

pub unsafe fn popstring() -> Result<String, Error> {
    if G_STACKTOP.is_null() || (*G_STACKTOP).is_null() {
        return Err(Error::InvalidStackError);
    }

    let mut string_wide: Vec<u16> = vec![0; G_STRINGSIZE as _];
    let th: *mut stack_t = *G_STACKTOP;
    lstrcpyW(string_wide.as_mut_ptr(), (*th).text.as_ptr() as _);
    let string = decode_wide(&string_wide)
        .to_str()
        .ok_or(Error::InvalidUnicode)?
        .to_string();
    *G_STACKTOP = (*th).next;
    GlobalFree(th as _);

    Ok(string)
}

pub unsafe fn pushint(int: i32) {
    pushstring(int.to_string())
}

pub fn encode_wide(string: impl AsRef<OsStr>) -> Vec<u16> {
    string.as_ref().encode_wide().chain(once(0)).collect()
}

pub fn decode_wide(mut wide_c_string: &[u16]) -> OsString {
    if let Some(null_pos) = wide_c_string.iter().position(|c| *c == 0) {
        wide_c_string = &wide_c_string[..null_pos];
    }

    OsString::from_wide(wide_c_string)
}
