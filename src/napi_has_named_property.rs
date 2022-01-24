use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_has_named_property(
  env: napi_env,
  value: napi_value,
  key: *const c_char,
  result: *mut bool,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = std::mem::transmute(value);
  let obj = value.to_object(env.scope).unwrap();
  let key = std::ffi::CStr::from_ptr(key).to_str().unwrap();
  let key = v8::String::new(env.scope, key).unwrap();
  *result = obj.has(env.scope, key.into()).unwrap_or(false);
  napi_ok
}
