use crate::env::{Env, EnvShared};
use crate::ffi::*;
use crate::napi_typeof::get_value_type;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_unwrap(
  env: napi_env,
  value: napi_value,
  result: *mut *mut c_void,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = std::mem::transmute(value);
  let obj = value.to_object(env.scope).unwrap();
  let shared = &*(env.shared as *const EnvShared);
  println!("unwrap napi_wrap: {:?} {:?}", shared.napi_wrap, get_value_type(shared.napi_wrap));
  let ext = obj.get(env.scope, shared.napi_wrap);
  println!("unwrap obj get: {:?}", ext);
  let ext = v8::Local::<v8::External>::try_from(ext.unwrap()).unwrap();
  *result = ext.value();
  napi_ok
}
