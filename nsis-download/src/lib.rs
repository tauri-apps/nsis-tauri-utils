#![allow(clippy::missing_safety_doc)]

use std::{fs, io, path::Path};

#[cfg(feature = "dylib")]
use nsis_utils::{exdll_init, popstring, pushint, stack_t, wchar_t};
#[cfg(feature = "dylib")]
use windows_sys::Win32::Foundation::HWND;

#[cfg(feature = "dylib")]
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

pub fn download_file(url: &str, path: &str) -> i32 {
    let path = Path::new(path);
    let _ = fs::remove_file(path);
    let _ = fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new("./")));

    let response = match ureq::get(url).call() {
        Ok(data) => data,
        Err(err) => {
            return match err {
                ureq::Error::Status(code, _) => code as i32,
                ureq::Error::Transport(_) => 499,
            }
        }
    };

    let mut reader = response.into_reader();

    if fs::File::create(path)
        .and_then(|mut file| io::copy(&mut reader, &mut file))
        .is_err()
        // Check if file was created
        || !Path::new(&path).exists()
    {
        return 1;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_downloads() {
        assert_eq!(
            download_file(
                "https://go.microsoft.com/fwlink/p/?LinkId=2124703",
                "wv2setup.exe"
            ),
            0
        )
    }
}
