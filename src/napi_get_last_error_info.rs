use crate::ffi::*;

#[napi_sym]
fn napi_get_last_error_info(
  env: napi_env,
  error_code: *mut *const napi_extended_error_info,
) -> Result {
  *error_code = std::ptr::null();
  Ok(())
}
