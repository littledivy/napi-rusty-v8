use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_get_boolean(
  env: napi_env,
  value: bool,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::Boolean::new(env.scope, value).into();
  *result = std::mem::transmute(value);
  napi_ok
}
