fn main() {
    if std::env::var("CARGO_FEATURE_TEST").as_deref() != Ok("1") {
        println!("cargo::rustc-link-arg=/ENTRY:DllMain")
    }
}
