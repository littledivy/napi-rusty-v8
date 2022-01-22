use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_get_global(
  env: napi_env,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let context = env.scope.get_current_context();
  let global = context.global(env.scope);
  let value: v8::Local<v8::Value> = global.into();
  *result = std::mem::transmute(value);
  napi_ok
}
