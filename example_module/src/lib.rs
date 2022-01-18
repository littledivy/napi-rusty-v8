use napi_sys::*;
use std::os::raw::c_void;
use std::os::raw::c_char;
use std::ptr;

#[no_mangle]
pub unsafe extern "C" fn hello(env: napi_env, info: napi_callback_info) -> napi_value {
    println!("Hello from Rust!");

    let mut result: napi_value = ptr::null_mut();
    napi_create_string_utf8(env, "world".as_ptr() as *const i8, 5, &mut result);
    assert!(!result.is_null());
    result
}

#[no_mangle]
unsafe extern "C" fn napi_register_module_v1(
  env: napi_env,
  exports: napi_value,
) -> napi_value {
  println!("[lib]: napi_register_module_v1");
  let prop = napi_property_descriptor {
    utf8name: "hello".as_ptr() as *const c_char,
    name: std::ptr::null_mut(), 
    method: Some(hello),
    getter: None,
    setter: None,
    value: std::ptr::null_mut(),
    attributes: 0,
    data: std::ptr::null_mut(),
  };

  napi_define_properties(env, exports, 1, &prop);
  std::mem::forget(prop);
  exports
}
