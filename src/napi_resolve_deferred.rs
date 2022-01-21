use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_resolve_deferred(
  env: napi_env,
  deferred: napi_deferred,
  result: napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let resolver: v8::Local<v8::PromiseResolver> = std::mem::transmute(deferred);
  resolver
    .resolve(env.scope, std::mem::transmute(result))
    .unwrap();
  napi_ok
}
