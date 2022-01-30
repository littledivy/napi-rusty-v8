use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[napi_sym]
fn napi_is_promise(
  env: napi_env,
  value: napi_value,
  result: *mut bool,
) -> Result {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = std::mem::transmute(value);
  *result = value.is_promise();
  Ok(())
}
