use crate::ffi::*;

#[napi_sym]
fn napi_escape_handle(
  env: napi_env,
  handle_scope: napi_escapable_handle_scope,
  escapee: napi_value,
  result: *mut napi_value,
) -> Result {
  // TODO
  Ok(())
}
