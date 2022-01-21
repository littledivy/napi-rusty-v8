use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_create_object(
  env: napi_env,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let object = v8::Object::new(env.scope);
  *result = std::mem::transmute(object);
  napi_ok
}
