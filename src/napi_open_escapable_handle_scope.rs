use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_open_escapable_handle_scope(
  env: napi_env,
  result: *mut napi_escapable_handle_scope,
) -> napi_status {
  // TODO: do this properly
  napi_ok
}
