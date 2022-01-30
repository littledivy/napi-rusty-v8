use crate::ffi::*;

pub const NAPI_VERSION: u32 = 8;

#[napi_sym]
fn napi_get_version(
  _: napi_env,
  version: *mut u32,
) -> Result {
  *version = NAPI_VERSION;
  Ok(())
}
