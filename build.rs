fn main() {
  println!("cargo:rustc-env=LINK=/DEF:{}", std::path::Path::new("exports.def").canonicalize().unwrap().display());
}
