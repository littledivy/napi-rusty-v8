use crate::env::Env;
use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_throw(
  env: napi_env,
  error: napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  let error = std::mem::transmute(error);
  env.scope.throw_exception(error);
  napi_ok
}
