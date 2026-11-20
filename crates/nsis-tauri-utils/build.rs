use std::io::Write;

fn main() {
    combine_plugins_and_write_to_out_dir();

    println!("cargo::rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Follow nsis-plugin's template; these flags apply only to the plugin DLL.
    let args: &[&str] = match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("msvc") => &[
            "/NODEFAULTLIB",
            "/ENTRY:DllMainCRTStartup",
            "/OPT:REF",
            "/OPT:ICF",
        ],
        _ => &["-nostartfiles", "-nodefaultlibs"],
    };
    for arg in args {
        println!("cargo::rustc-link-arg-cdylib={arg}");
    }

    // The aborting runtime has no SEH handlers or CRT load configuration.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
        && std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("x86")
    {
        println!("cargo::rustc-link-arg-cdylib=/SAFESEH:NO");
    }
}

/// Combines the plugins into one file that is included in lib.rs
/// using `include!(concat!(env!("OUT_DIR"), "/combined_libs.rs"));`
///
/// Plugins are combined this way because it saves a few kilobytes in the generated DLL
/// than the making nsis-tauri-utils depend on other plugins and re-export the DLLs
fn combine_plugins_and_write_to_out_dir() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let path = format!("{out_dir}/combined_libs.rs");
    let mut file = std::fs::File::create(path).unwrap();
    for (source, plugin) in [
        (
            "../nsis-semvercompare/src/lib.rs",
            include_str!("../nsis-semvercompare/src/lib.rs"),
        ),
        (
            "../nsis-process/src/lib.rs",
            include_str!("../nsis-process/src/lib.rs"),
        ),
        (
            "../nsis-string/src/lib.rs",
            include_str!("../nsis-string/src/lib.rs"),
        ),
    ] {
        println!("cargo::rerun-if-changed={source}");
        let lines = plugin
            .lines()
            .skip_while(|&l| l != "/* start-marker */")
            .take_while(|&l| l != "/* end-marker */")
            .collect::<Vec<&str>>();

        // skip last line which should be #[cfg(test)]
        let content = lines[..lines.len() - 1].join("\n");
        file.write_all(content.as_bytes()).unwrap();
    }
}
