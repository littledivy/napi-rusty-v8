use crate::env::Env;
use crate::ffi::*;
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
  // TODO: use private fields
  let key = v8::String::new(env.scope, "__native_object__").unwrap();
  let ext = obj.get(env.scope, key.into()).unwrap();
  let ext = v8::Local::<v8::External>::try_from(ext).unwrap();
  *result = ext.value();
  napi_ok
}
