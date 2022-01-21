use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_create_bigint_uint64(
  env: napi_env,
  value: u64,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let value: v8::Local<v8::Value> =
    v8::BigInt::new_from_u64(env.scope, value).into();
  *result = std::mem::transmute(value);
  napi_ok
}
