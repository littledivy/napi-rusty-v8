use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_get_property(
  env: napi_env,
  object: napi_value,
  key: napi_value,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let object: v8::Local<v8::Value> = std::mem::transmute(object);
  let object = object.to_object(env.scope).unwrap();
  let key: v8::Local<v8::Value> = std::mem::transmute(key);
  let value: v8::Local<v8::Value> = object.get(env.scope, key).unwrap();
  *result = std::mem::transmute(value);
  napi_ok
}
