use crate::ffi::*;

thread_local! {
  static RELEASE: *const c_char = {
    let mut release = std::ffi::CString::new("Deno N-API").unwrap();
    release.as_ptr()
  };

  static NODE_VERSION: napi_node_version = {
    let release = std::ffi::CString::new("Deno N-API").unwrap();
    let release = release.as_ptr();
    std::mem::forget(release);
    napi_node_version {
      major: 17,
      minor: 4,
      patch: 0,
      release: release,
    }
  }
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_node_version(
  _: napi_env,
  result: *mut *const napi_node_version,
) -> napi_status {
  NODE_VERSION.with(|version| {
    *result = version as *const napi_node_version;
  });
  napi_ok
}
