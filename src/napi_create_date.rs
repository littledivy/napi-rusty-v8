use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_create_date(
  env: napi_env,
  time: f64,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> =
    v8::Date::new(env.scope, time).unwrap().into();
  *result = std::mem::transmute(value);
  napi_ok
}
