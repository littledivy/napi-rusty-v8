use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_strict_equals(
  env: napi_env,
  lhs: napi_value,
  rhs: napi_value,
  result: *mut bool,
) -> napi_status {
  let mut env = &mut (env as *mut Env);
  let lhs: v8::Local<v8::Value> = std::mem::transmute(lhs);
  let rhs: v8::Local<v8::Value> = std::mem::transmute(rhs);
  *result = lhs.strict_equals(rhs);
  napi_ok
}
