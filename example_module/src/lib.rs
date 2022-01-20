#[macro_use]
extern crate napi_derive;

// use napi::bindgen_prelude::*;

#[napi]
fn hello() {
  println!("Hello from Rust!");
  // return String::from("world");
}
