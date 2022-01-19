extern crate napi_build;

fn main() {
  println!("cargo:rustc-cdylib-link-arg=-rdynamic");
  println!("cargo:rustc-cdylib-link-arg=-undefined");
  println!("cargo:rustc-cdylib-link-arg=-Wl,-export-dynamic");
  napi_build::setup();
}
