fn main() {
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
