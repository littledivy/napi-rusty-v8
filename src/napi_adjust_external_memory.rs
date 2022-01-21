use crate::env::Env;
use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_adjust_external_memory(
  env: napi_env,
  change_in_bytes: i64,
  adjusted_value: *mut i64,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  todo!();
}
