use crate::env::Env;
use crate::ffi::*;

#[napi_sym]
fn napi_create_string_utf16(
  env: napi_env,
  string: *const u16,
  length: usize,
  result: *mut napi_value,
) -> Result {
  let mut env = &mut *(env as *mut Env);
  let string = std::slice::from_raw_parts(string, length);
  let v8str =
    v8::String::new_from_two_byte(env.scope, string, v8::NewStringType::Normal)
      .unwrap();
  let value: v8::Local<v8::Value> = v8str.into();
  *result = std::mem::transmute(value);
  Ok(())
}
