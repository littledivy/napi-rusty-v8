#[macro_use]
extern crate napi_derive;

// use napi::bindgen_prelude::*;

#[napi]
fn hello(name: String) -> String {
  println!("Hello, {}!", name);
  String::from("World")
}

#[napi]
fn add(a: i32, b: i32) -> i32 {
  a + b
}
