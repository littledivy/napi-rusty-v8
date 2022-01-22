use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_instanceof(
  env: napi_env,
  value: napi_value,
  constructor: napi_value,
  result: *mut bool,
) -> napi_status {
  let mut env = &mut (env as *mut Env);
  let value: v8::Local<v8::Value> = std::mem::transmute(value);
  let constructor: v8::Local<v8::Value> = std::mem::transmute(constructor);
  // TODO: what is the rusty v8 API
  // *result = value.instance_of(constructor);
  napi_ok
}
