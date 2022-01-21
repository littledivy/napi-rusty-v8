use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_reject_deferred(
  env: napi_env,
  deferred: napi_deferred,
  error: napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let resolver: v8::Local<v8::PromiseResolver> = std::mem::transmute(deferred);
  resolver.reject(env.scope, std::mem::transmute(error)).unwrap();
  napi_ok
}
