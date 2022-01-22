#![allow(non_camel_case_types)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

#[cfg(unix)]
use libloading::os::unix::*;

#[cfg(windows)]
use libloading::os::windows::*;

pub mod util;
pub mod callback_info;
pub mod env;
pub mod ffi;
pub mod napi_add_env_cleanup_hook;
pub mod napi_adjust_external_memory;
pub mod napi_call_threadsafe_function;
pub mod napi_create_bigint_int64;
pub mod napi_create_bigint_uint64;
pub mod napi_create_bigint_words;
pub mod napi_create_double;
pub mod napi_create_error;
pub mod napi_create_external_buffer;
pub mod napi_create_function;
pub mod napi_create_int32;
pub mod napi_create_int64;
pub mod napi_create_object;
pub mod napi_create_promise;
pub mod napi_create_reference;
pub mod napi_create_string_utf8;
pub mod napi_create_threadsafe_function;
pub mod napi_create_uint32;
pub mod napi_define_class;
pub mod napi_define_properties;
pub mod napi_delete_reference;
pub mod napi_get_and_clear_last_exception;
pub mod napi_get_cb_info;
pub mod napi_get_reference_value;
pub mod napi_get_undefined;
pub mod napi_get_value_int32;
pub mod napi_get_value_string_utf8;
pub mod napi_is_promise;
pub mod napi_module_register;
pub mod napi_new_instance;
pub mod napi_reject_deferred;
pub mod napi_release_threadsafe_function;
pub mod napi_resolve_deferred;
pub mod napi_set_named_property;
pub mod napi_throw;
pub mod napi_throw_error;
pub mod napi_wrap;
pub mod napi_get_value_bool;
pub mod napi_get_property_names;
pub mod napi_get_named_property;
pub mod napi_typeof;
pub mod napi_unwrap;
pub mod napi_get_version;
pub mod napi_get_null;
pub mod napi_get_global;
pub mod napi_get_boolean;
pub mod napi_get_value_double;
pub mod napi_create_array_with_length;
pub mod napi_get_array_length;
pub mod napi_get_new_target;
pub mod napi_coerce_to_object;
pub mod napi_coerce_to_string;
pub mod napi_is_exception_pending;
pub mod napi_get_value_external;
pub mod napi_close_escapable_handle_scope;
pub mod napi_open_escapable_handle_scope;
pub mod napi_open_handle_scope;
pub mod napi_close_handle_scope;
pub mod napi_is_arraybuffer;
pub mod napi_is_buffer;
pub mod napi_is_error;
pub mod napi_is_array;
pub mod napi_create_type_error;
pub mod napi_create_range_error;
pub mod napi_create_arraybuffer;
pub mod napi_get_arraybuffer_info;
pub mod napi_create_buffer;
pub mod napi_get_buffer_info;
pub mod napi_create_external;
pub mod napi_call_function;
pub mod napi_set_property;
pub mod napi_get_property;
pub mod napi_set_element;
pub mod napi_get_element;
pub mod napi_escape_handle;
pub mod napi_reference_ref;
pub mod napi_reference_unref;
pub mod napi_strict_equals;
pub mod napi_create_external_arraybuffer;
pub mod napi_run_script;
pub mod napi_ref_threadsafe_function;
pub mod napi_unref_threadsafe_function;
pub mod napi_create_date;
pub mod napi_get_date_value;
pub mod napi_is_date;
pub mod napi_get_all_property_names;
pub mod napi_get_instance_data;
pub mod napi_set_instance_data;
pub mod napi_get_value_uint32;
pub mod napi_create_async_work;
pub mod napi_queue_async_work;
pub mod napi_delete_async_work;
pub mod napi_cancel_async_work;

use deno_core::JsRuntime;

use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

fn main() {
  let mut runtime = JsRuntime::new(Default::default());

  let isolate = runtime.v8_isolate();

  let mut scope = &mut runtime.handle_scope();
  let context = scope.get_current_context();
  let inner_scope = &mut v8::ContextScope::new(scope, context);
  let global = context.global(inner_scope);

  let mut exports = v8::Object::new(inner_scope);
  let mut env = Env { scope: inner_scope };

  #[cfg(unix)]
  let flags = RTLD_LAZY;
  #[cfg(not(unix))]
  let flags = 0x00000008;

  // Initializer callback.
  #[cfg(unix)]
  let library = unsafe {
    Library::open(
      // Some("./example_module/target/release/libexample_module.so"),
      Some("./testdata/node_modules/dprint-node/dprint-node.linux-x64-gnu.node"),
      flags,
    )
    .unwrap()
  };

  #[cfg(not(unix))]
  let library = unsafe {
    Library::load_with_flags(
      "./example_module/target/release/example_module.dll",
      flags,
    )
    .unwrap()
  };

  let init = unsafe {
    library.get::<unsafe extern "C" fn(env: napi_env, exports: napi_value) -> napi_value>(b"napi_register_module_v1")
  };

  if let Ok(init) = init {
    unsafe {
      init(
        &mut env as *mut _ as *mut c_void,
        std::mem::transmute(exports),
      )
    };
  } else {
    eprintln!("Failed to init module! napi_register_module_v1 symbol not found");
  }

  let exports_str = v8::String::new(inner_scope, "exports").unwrap();

  global
    .set(inner_scope, exports_str.into(), exports.into())
    .unwrap();

  let source = std::fs::read_to_string(std::env::args().nth(1).unwrap_or(String::from("test.js"))).unwrap();

  let script = v8::String::new(
    inner_scope,
    &source,
  )
  .unwrap();

  let script = v8::Script::compile(inner_scope, script, None)
    .expect("failed to compile script");

  script.run(inner_scope).unwrap();
}
