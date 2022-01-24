use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_escape_handle(
  env: napi_env,
  handle_scope: napi_escapable_handle_scope,
  escapee: napi_value,
  result: *mut napi_value,
) -> napi_status {
  // TODO
  napi_ok
}
