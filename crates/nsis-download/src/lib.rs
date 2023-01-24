use std::{fs, io, path::Path};

use pluginapi::{exdll_init, popstring, pushint, stack_t, wchar_t};
use progress_streams::ProgressReader;
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{
        Controls::{PBM_SETPOS, PROGRESS_CLASSW, WC_STATICW},
        WindowsAndMessaging::{
            CreateWindowExW, FindWindowExW, GetWindowLongPtrW, SendMessageW, SetWindowPos,
            SetWindowTextW, ShowWindow, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOSIZE, SW_HIDE,
            WM_GETFONT, WM_SETFONT, WS_CHILD, WS_VISIBLE,
        },
    },
};

/// Download a file from an URL to a path.
///
/// # Safety
///
/// This function always expects 2 strings on the stack ($1: url, $2: path) and will panic otherwise.
#[no_mangle]
pub unsafe extern "C" fn Download(
    hwnd_parent: HWND,
    string_size: u32,
    variables: *mut wchar_t,
    stacktop: *mut *mut stack_t,
) {
    exdll_init(string_size, variables, stacktop);

    let url = popstring().unwrap();
    let path = popstring().unwrap();

    let status = download_file(hwnd_parent, &url, &path);
    pushint(status);
}

fn download_file(hwnd_parent: HWND, url: &str, path: &str) -> i32 {
    let childhwnd;
    let mut progress_bar: HWND = 0;
    let mut progress_text: HWND = 0;
    let mut downloading_text: HWND = 0;
    let mut details_section: HWND = 0;
    let mut details_section_resized = false;
    let mut details_section_resized_back = false;

    if hwnd_parent != 0 {
        childhwnd = find_window(hwnd_parent, "#32770");
        if childhwnd != 0 {
            details_section = find_window(childhwnd, "SysListView32");
            let expanded = is_visible(details_section);
            unsafe {
                progress_bar = CreateWindowExW(
                    0,
                    PROGRESS_CLASSW,
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE,
                    0,
                    if expanded { 40 } else { 75 },
                    450,
                    18,
                    childhwnd,
                    0,
                    0,
                    std::ptr::null(),
                );

                downloading_text = CreateWindowExW(
                    0,
                    WC_STATICW,
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE,
                    0,
                    if expanded { 60 } else { 95 },
                    450,
                    18,
                    childhwnd,
                    0,
                    0,
                    std::ptr::null(),
                );

                progress_text = CreateWindowExW(
                    0,
                    WC_STATICW,
                    std::ptr::null(),
                    WS_CHILD | WS_VISIBLE,
                    0,
                    if expanded { 78 } else { 113 },
                    450,
                    18,
                    childhwnd,
                    0,
                    0,
                    std::ptr::null(),
                );

                let font = SendMessageW(childhwnd, WM_GETFONT, 0, 0);
                SendMessageW(downloading_text, WM_SETFONT, font as _, 0);
                SendMessageW(progress_text, WM_SETFONT, font as _, 0);
            };
        }
    }

    let response = match ureq::get(url).call() {
        Ok(data) => data,
        Err(err) => {
            return match err {
                ureq::Error::Status(code, _) => code as i32,
                ureq::Error::Transport(_) => 499,
            }
        }
    };

    let total = response
        .header("Content-Length")
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap();

    let mut read = 0;

    let mut reader = response.into_reader();
    let mut reader = ProgressReader::new(&mut reader, |progress: usize| {
        let expanded = is_visible(details_section);
        if expanded && !details_section_resized {
            unsafe {
                SetWindowPos(progress_bar, 0, 0, 40, 0, 0, SWP_NOSIZE);
                SetWindowPos(downloading_text, 0, 0, 60, 0, 0, SWP_NOSIZE);
                SetWindowPos(progress_text, 0, 0, 78, 0, 0, SWP_NOSIZE);

                SetWindowPos(details_section, 0, 0, 100, 450, 120, SWP_FRAMECHANGED);
            }
            details_section_resized = true;
        }

        read += progress;

        let percentage = (read as f64 / total as f64) * 100.0;
        unsafe { SendMessageW(progress_bar, PBM_SETPOS, percentage as _, 0) };

        let text = pluginapi::encode_wide(format!(
            "{} / {} KiB  - {:.2}%",
            read / 1024,
            total / 1024,
            percentage,
        ));
        unsafe { SetWindowTextW(progress_text, text.as_ptr()) };

        let text = pluginapi::encode_wide(format!("Downloading {} ...", url));
        unsafe { SetWindowTextW(downloading_text, text.as_ptr()) };

        if percentage >= 100. && !details_section_resized_back {
            unsafe {
                ShowWindow(progress_bar, SW_HIDE);
                ShowWindow(progress_text, SW_HIDE);
                ShowWindow(downloading_text, SW_HIDE);
                SetWindowPos(details_section, 0, 0, 41, 450, 180, SWP_FRAMECHANGED);
            }
            details_section_resized_back = true;
        }
    });

    let path = Path::new(path);
    fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new("."))).unwrap();

    let mut file = fs::File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .unwrap();

    let res = io::copy(&mut reader, &mut file);

    i32::from(res.is_err())
}

fn find_window(parent: HWND, class: impl AsRef<str>) -> HWND {
    let class = pluginapi::encode_wide(class.as_ref());
    unsafe { FindWindowExW(parent, 0, class.as_ptr(), std::ptr::null()) }
}

fn is_visible(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
    (style & !WS_VISIBLE as i32) != style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_downloads() {
        assert_eq!(
            download_file(
                0,
                "https://go.microsoft.com/fwlink/p/?LinkId=2124703",
                "wv2setup.exe"
            ),
            0
        )
    }
}
