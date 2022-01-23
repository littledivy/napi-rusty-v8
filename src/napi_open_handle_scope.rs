use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_open_handle_scope(
  env: napi_env,
  result: *mut napi_handle_scope,
) -> napi_status {
  // TODO: do this properly
  napi_ok
}
