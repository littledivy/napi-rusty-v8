use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_set_named_property(
  env: napi_env,
  object: napi_value,
  name: *const c_char,
  value: napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let object: v8::Local<v8::Object> = std::mem::transmute(object);
  let name = CStr::from_ptr(name).to_str().unwrap();
  let value: v8::Local<v8::Value> = std::mem::transmute(value);

  let name = v8::String::new(env.scope, name).unwrap();
  object.set(env.scope, name.into(), value).unwrap();

  napi_ok
}
