#![allow(non_camel_case_types)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

#[cfg(unix)]
use libloading::os::unix::*;

#[cfg(windows)]
use libloading::os::windows::*;

pub mod callback_info;
pub mod env;
pub mod ffi;
pub mod napi_adjust_external_memory;
pub mod napi_create_bigint_int64;
pub mod napi_create_bigint_uint64;
pub mod napi_create_bigint_words;
pub mod napi_create_double;
pub mod napi_create_error;
pub mod napi_create_function;
pub mod napi_create_int32;
pub mod napi_create_int64;
pub mod napi_create_object;
pub mod napi_create_reference;
pub mod napi_create_string_utf8;
pub mod napi_create_uint32;
pub mod napi_define_class;
pub mod napi_define_properties;
pub mod napi_get_and_clear_last_exception;
pub mod napi_get_cb_info;
pub mod napi_get_undefined;
pub mod napi_get_value_int32;
pub mod napi_get_value_string_utf8;
pub mod napi_module_register;
pub mod napi_new_instance;
pub mod napi_set_named_property;
pub mod napi_throw;
pub mod napi_throw_error;
pub mod napi_wrap;

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
      Some("./example_module/target/release/libexample_module.so"),
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
    library.get::<unsafe extern "C" fn(env: napi_env, exports: napi_value) -> napi_value>(b"napi_register_module_v1").unwrap()
  };

  unsafe {
    init(
      &mut env as *mut _ as *mut c_void,
      std::mem::transmute(exports),
    )
  };

  let exports_str = v8::String::new(inner_scope, "exports").unwrap();

  global
    .set(inner_scope, exports_str.into(), exports.into())
    .unwrap();

  let script = v8::String::new(
    inner_scope,
    r#"
    function print(txt) {
      Deno.core.print(txt + "\n");
    }

    print(exports.hello("Rust"));
    print(exports.add(1, 2));
    "#,
  )
  .unwrap();

  let script = v8::Script::compile(inner_scope, script, None)
    .expect("failed to compile script");

  script.run(inner_scope).unwrap();
}
