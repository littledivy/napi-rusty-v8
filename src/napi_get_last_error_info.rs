use crate::ffi::*;

#[no_mangle]
pub unsafe extern "C" fn napi_get_last_error_info(
  env: napi_env,
  error_code: *mut *const napi_extended_error_info,
) -> napi_status {
  *error_code = std::ptr::null();
  napi_ok
}
