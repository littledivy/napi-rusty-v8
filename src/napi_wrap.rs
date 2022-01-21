use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_wrap(
  env: napi_env,
  value: *mut v8::Value,
  native_object: *mut c_void,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = std::mem::transmute(value);
  let obj = value.to_object(env.scope).unwrap();
  // TODO: use private fields
  let key = v8::String::new(env.scope, "__native_object__").unwrap();
  let ext = v8::External::new(env.scope, native_object);
  obj.set(env.scope, key.into(), ext.into());
  napi_ok
}
