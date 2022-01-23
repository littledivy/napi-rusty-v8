use crate::env::{Env, EnvShared};
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
  let shared = &*(env.shared as *const EnvShared);
  let napi_wrap = v8::Local::new(env.scope, &shared.napi_wrap);
  let ext = v8::External::new(env.scope, native_object);
  obj.set_private(env.scope, napi_wrap, ext.into());
  napi_ok
}
