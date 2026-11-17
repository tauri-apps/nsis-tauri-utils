fn main() {
    println!("cargo::rustc-link-arg-cdylib=/ENTRY:DllMain");
}
