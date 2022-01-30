use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[napi_sym]
fn napi_get_value_uint32(
  env: napi_env,
  value: napi_value,
  result: *mut u32,
) -> Result {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> = std::mem::transmute(value);
  *result = value.uint32_value(env.scope).unwrap();
  Ok(())
}
