use crate::env::{Env, EnvShared};
use crate::ffi::*;
use crate::napi_typeof::get_value_type;
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
  println!("napi_wrap: {:?} {:?}", shared.napi_wrap, get_value_type(shared.napi_wrap));
  let ext = v8::External::new(env.scope, native_object);
  println!("ext: {:?}", ext.value());
  obj.set(env.scope, shared.napi_wrap, ext.into());
  let v = obj.get(env.scope, shared.napi_wrap);
  let v_ext = v8::Local::<v8::External>::try_from(v.unwrap()).unwrap();
  println!("wrap obj get {:?} {:?}", v_ext.value(), get_value_type(v.unwrap()));
  napi_ok
}
