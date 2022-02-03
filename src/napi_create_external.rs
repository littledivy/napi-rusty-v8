use crate::env::Env;
use crate::ffi::*;

#[napi_sym]
fn napi_create_external(
  env: napi_env,
  value: *mut c_void,
  finalize_cb: napi_finalize,
  finalize_hint: *mut c_void,
  result: *mut napi_value,
) -> Result {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = v8::External::new(env.scope, value).into();
  // TODO: finalization
  *result = std::mem::transmute(value);
  Ok(())
}
