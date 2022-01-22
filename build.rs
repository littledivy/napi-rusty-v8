fn make_exports() -> std::io::Result<()> {
  let mut exports = String::from("LIBRARY\nEXPORTS\n");
  for entry in std::fs::read_dir("./src")? {
    let entry = entry?;
    if let Ok(ftype) = entry.file_type() {
      let name = entry.file_name();
      let name = name.to_str().unwrap();
      if ftype.is_file() && name.starts_with("napi_") && name.ends_with(".rs") {
        exports.push_str(&format!("  {}\n", &name[0..name.len() - 3]));
      }
    }
  }
  std::fs::write("./exports.def", exports)?;
  Ok(())
}

fn main() {
  make_exports().unwrap();
  println!(
    "cargo:rustc-env=LINK=/DEF:{}",
    std::path::Path::new("exports.def")
      .canonicalize()
      .unwrap()
      .display()
  );
}
