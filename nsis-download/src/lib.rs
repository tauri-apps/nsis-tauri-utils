#![allow(clippy::missing_safety_doc)]

use std::{fs, io, path::Path};

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

    match download_file(&url, &path) {
        Ok(_) => pushint(0),
        Err(_) => pushint(1),
    }
}

fn download_file(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let _ = fs::remove_file(path);
    fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new("./")))?;

    let response = ureq::get(url).call()?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(path)?;
    io::copy(&mut reader, &mut file)?;

    if !Path::new(&path).exists() {
        return Err(Box::new(io::Error::new(io::ErrorKind::NotFound, "")));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_downloads() {
        assert!(download_file(
            "https://go.microsoft.com/fwlink/p/?LinkId=2124703",
            "wv2setup.exe"
        )
        .is_ok())
    }
}
