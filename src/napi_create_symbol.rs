use crate::env::Env;
use crate::ffi::*;

// TODO: properly implement
#[no_mangle]
pub unsafe extern "C" fn napi_create_symbol(
  env: napi_env,
  description: napi_value,
  result: *mut napi_value,
) -> napi_status {
  let mut _env = &mut *(env as *mut Env);
  napi_ok
}
