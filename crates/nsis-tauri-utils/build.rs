use std::io::Write;

fn main() {
    combine_plugins_and_write_to_out_dir();
    if std::env::var("CARGO_FEATURE_TEST").as_deref() != Ok("1") {
        println!("cargo::rustc-link-arg=/ENTRY:DllMain")
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
    for plugin in [
        include_str!("../nsis-semvercompare/src/lib.rs"),
        include_str!("../nsis-process/src/lib.rs"),
        include_str!("../nsis-string/src/lib.rs"),
    ] {
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
