#[macro_use]
extern crate napi_derive;

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
struct Point {
  x: i32,
  y: i32,
}

#[napi]
impl Point {
  #[napi(factory)]
  pub fn new(x: i32, y: i32) -> Point {
    Point { x, y }
  }
}
