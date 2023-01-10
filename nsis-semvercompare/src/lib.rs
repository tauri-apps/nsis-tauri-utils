#![allow(clippy::missing_safety_doc)]

use std::str::FromStr;

use semver::Version;
use shared::{exdll_init, popstring, pushint, stack_t, wchar_t};
use windows_sys::Win32::Foundation::HWND;

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

fn semver_compare(v1: &str, v2: &str) -> i32 {
    let v1 = Version::from_str(v1);
    let v2 = Version::from_str(v2);

    let (v1, v2) = match (v1, v2) {
        (Ok(_), Err(_)) => return 1,
        (Err(_), Err(_)) => return 0,
        (Err(_), Ok(_)) => return -1,
        (Ok(v1), Ok(v2)) => (v1, v2),
    };

    #[allow(clippy::comparison_chain)]
    if v1 > v2 {
        1
    } else if v1 == v2 {
        0
    } else if v1 < v2 {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        for (v1, v2, ret) in [
            ("1.2.1", "1.2.0", 1),
            ("1.2.0", "1.2.1", -1),
            ("1.2.1", "1.2.1", 0),
            ("1.2.1-alpha.1", "1.2.1-beta.5", -1),
            ("1.2.1-rc.1", "1.2.1-beta.1", 1),
            ("1.2.1-alpha.1", "1.2.1-alpha.1", 0),
            ("1.2qe2.1-alpha.1", "1.2.1-alpha.1", -1),
            ("1.2.1-alpha.1", "-q1.2.1-alpha.1", 1),
            ("1.2.saf1-alpha.1", "-q1.2.1-alpha.1", 0),
            ("1.0.0-aluc.0", "1.0.0", -1),
            (" 1.0.0-aluc.1", "1.0.0-bdfsf.0", -1),
            ("1.2.1-fffasd.1", "1.2.1-dasdqwe.1", 1),
            ("1.2.1-gasfdlkj.1", "1.2.1-calskjd.1", 1),
        ] {
            assert_eq!(semver_compare(v1, v2), ret);
        }
    }
}
