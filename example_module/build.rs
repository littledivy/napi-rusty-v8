fn main() {
  println!("cargo:rustc-cdylib-link-arg=-rdynamic");
  println!("cargo:rustc-cdylib-link-arg=-undefined");

    println!("cargo:rustc-cdylib-link-arg=-Wl,-export-dynamic");
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}
