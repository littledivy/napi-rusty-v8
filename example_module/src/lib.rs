#[macro_use]
extern crate napi_derive;

use futures::prelude::*;
use napi::bindgen_prelude::*;
use tokio::fs;

#[napi]
fn hello(name: String) -> String {
  println!("Hello, {}!", name);
  String::from("World")
}

#[napi]
fn add(a: i32, b: i32) -> i32 {
  a + b
}

#[napi(js_name = "Point")]
pub struct Point {
  pub x: i32,
  pub y: i32,
}

#[napi]
impl Point {
  #[napi(constructor)]
  pub fn new(x: i32, y: i32) -> Point {
    Point { x, y }
  }
}

#[napi]
async fn read_file_async(path: String) -> Result<Buffer> {
  fs::read(path)
    .map(|r| match r {
      Ok(content) => Ok(content.into()),
      Err(e) => Err(Error::new(
        Status::GenericFailure,
        format!("failed to read file, {}", e),
      )),
    })
    .await
}
