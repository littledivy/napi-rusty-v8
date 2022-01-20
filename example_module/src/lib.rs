#[macro_use]
extern crate napi_derive;

// use napi::bindgen_prelude::*;

#[napi]
fn hello(name: String) -> String {
  println!("Hello, {}!", name);
  return String::from("test");
}
