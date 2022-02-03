use crate::env::Env;
use crate::ffi::*;

#[napi_sym]
fn napi_is_date(env: napi_env, value: napi_value, result: *mut bool) -> Result {
  let mut env = &mut (env as *mut Env);
  let value: v8::Local<v8::Value> = transmute(value);
  *result = value.is_date();
  Ok(())
}
