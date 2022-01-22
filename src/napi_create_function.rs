use crate::env::Env;
use crate::ffi::*;
use crate::function::create_function;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_create_function(
  env: napi_env,
  name: *const u8,
  length: usize,
  cb: napi_callback,
  cb_info: napi_callback_info,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let name = std::slice::from_raw_parts(name, length);
  let name = std::str::from_utf8(name).unwrap();
  let function = create_function(env, Some(name), cb, cb_info);
  let value: v8::Local<v8::Value> = function.into();
  *result = std::mem::transmute(value);
  napi_ok
}
