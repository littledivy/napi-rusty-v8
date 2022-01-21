use crate::env::Env;
use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_is_exception_pending(
  env: napi_env,
  result: *mut bool,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);
  // TODO
  *result = false;
  napi_ok
}
