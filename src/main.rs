#![allow(non_camel_case_types)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::ffi::CString;

use env::EnvShared;
#[cfg(unix)]
use libloading::os::unix::*;

#[cfg(windows)]
use libloading::os::windows::*;

pub mod env;
pub mod ffi;
pub mod function;
pub mod napi_add_env_cleanup_hook;
pub mod napi_adjust_external_memory;
pub mod napi_call_function;
pub mod napi_call_threadsafe_function;
pub mod napi_cancel_async_work;
pub mod napi_close_escapable_handle_scope;
pub mod napi_close_handle_scope;
pub mod napi_coerce_to_bool;
pub mod napi_coerce_to_number;
pub mod napi_coerce_to_object;
pub mod napi_coerce_to_string;
pub mod napi_create_array_with_length;
pub mod napi_create_arraybuffer;
pub mod napi_create_async_work;
pub mod napi_create_bigint_int64;
pub mod napi_create_bigint_uint64;
pub mod napi_create_bigint_words;
pub mod napi_create_buffer_copy;
pub mod napi_create_buffer;
pub mod napi_create_date;
pub mod napi_create_double;
pub mod napi_create_error;
pub mod napi_create_external;
pub mod napi_create_external_arraybuffer;
pub mod napi_create_external_buffer;
pub mod napi_create_function;
pub mod napi_create_int32;
pub mod napi_create_int64;
pub mod napi_create_object;
pub mod napi_create_promise;
pub mod napi_create_range_error;
pub mod napi_create_reference;
pub mod napi_create_string_utf8;
pub mod napi_create_symbol;
pub mod napi_create_threadsafe_function;
pub mod napi_create_type_error;
pub mod napi_create_uint32;
pub mod napi_define_class;
pub mod napi_define_properties;
pub mod napi_delete_async_work;
pub mod napi_delete_reference;
pub mod napi_detach_arraybuffer;
pub mod napi_escape_handle;
pub mod napi_get_all_property_names;
pub mod napi_get_and_clear_last_exception;
pub mod napi_get_array_length;
pub mod napi_get_arraybuffer_info;
pub mod napi_get_boolean;
pub mod napi_get_buffer_info;
pub mod napi_get_cb_info;
pub mod napi_get_dataview_info;
pub mod napi_get_date_value;
pub mod napi_get_element;
pub mod napi_get_global;
pub mod napi_get_instance_data;
pub mod napi_get_named_property;
pub mod napi_get_new_target;
pub mod napi_get_null;
pub mod napi_get_property;
pub mod napi_get_property_names;
pub mod napi_get_prototype;
pub mod napi_get_reference_value;
pub mod napi_get_typedarray_info;
pub mod napi_get_undefined;
pub mod napi_get_value_bool;
pub mod napi_get_value_double;
pub mod napi_get_value_external;
pub mod napi_get_value_int32;
pub mod napi_get_value_string_utf8;
pub mod napi_get_value_uint32;
pub mod napi_get_version;
pub mod napi_instanceof;
pub mod napi_is_array;
pub mod napi_is_arraybuffer;
pub mod napi_is_buffer;
pub mod napi_is_dataview;
pub mod napi_is_date;
pub mod napi_is_detached_arraybuffer;
pub mod napi_is_error;
pub mod napi_is_exception_pending;
pub mod napi_is_promise;
pub mod napi_is_typedarray;
pub mod napi_module_register;
pub mod napi_new_instance;
pub mod napi_open_escapable_handle_scope;
pub mod napi_open_handle_scope;
pub mod napi_queue_async_work;
pub mod napi_ref_threadsafe_function;
pub mod napi_reference_ref;
pub mod napi_reference_unref;
pub mod napi_reject_deferred;
pub mod napi_release_threadsafe_function;
pub mod napi_resolve_deferred;
pub mod napi_run_script;
pub mod napi_set_element;
pub mod napi_set_instance_data;
pub mod napi_set_named_property;
pub mod napi_set_property;
pub mod napi_strict_equals;
pub mod napi_throw;
pub mod napi_throw_error;
pub mod napi_typeof;
pub mod napi_unref_threadsafe_function;
pub mod napi_unwrap;
pub mod napi_wrap;
pub mod node_api_get_module_file_name;
pub mod util;

use deno_core::JsRuntime;

use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

fn main() {
  let mut runtime = JsRuntime::new(Default::default());

  {
    let isolate = runtime.v8_isolate();

    let mut scope = &mut runtime.handle_scope();
    let context = scope.get_current_context();
    let inner_scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(inner_scope);

    let dlopen_func = v8::Function::builder(
      |handle_scope: &mut v8::HandleScope,
       args: v8::FunctionCallbackArguments,
       mut rv: v8::ReturnValue| {
        let context = v8::Context::new(handle_scope);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let napi_wrap_name = v8::String::new(scope, "napi_wrap").unwrap();
        let napi_wrap = v8::Private::new(scope, Some(napi_wrap_name));
        let napi_wrap = v8::Local::new(scope, napi_wrap);
        let napi_wrap = v8::Global::new(scope, napi_wrap);

        let path = args.get(0).to_string(scope).unwrap();
        let path = path.to_rust_string_lossy(scope);

        let mut exports = v8::Object::new(scope);

        // We need complete control over the env object's lifetime
        // so we'll use explicit allocation for it, so that it doesn't
        // die before the module itself. Using struct & their pointers
        // resulted in a use-after-free situation which turned out to be
        // unfixable, so here we are.
        let env_shared_ptr = unsafe {
          std::alloc::alloc(std::alloc::Layout::new::<EnvShared>())
            as *mut EnvShared
        };
        let mut env_shared = EnvShared::new(napi_wrap);
        let cstr = CString::new(path.clone()).unwrap();
        env_shared.filename = cstr.as_ptr();
        std::mem::forget(cstr);
        unsafe {
          env_shared_ptr.write(env_shared);
        }

        let env_ptr = unsafe {
          std::alloc::alloc(std::alloc::Layout::new::<Env>()) as napi_env
        };
        let mut env = Env::new(scope);
        env.shared = env_shared_ptr;
        unsafe {
          (env_ptr as *mut Env).write(env);
        }

        #[cfg(unix)]
        let flags = RTLD_LAZY;
        #[cfg(not(unix))]
        let flags = 0x00000008;

        #[cfg(unix)]
        let library = match unsafe { Library::open(Some(&path), flags) } {
          Ok(lib) => lib,
          Err(e) => {
            let message = v8::String::new(scope, &e.to_string()).unwrap();
            let error = v8::Exception::type_error(scope, message);
            scope.throw_exception(error);
            return;
          }
        };

        #[cfg(not(unix))]
        let library = match unsafe { Library::load_with_flags(&path, flags) } {
          Ok(lib) => lib,
          Err(e) => {
            let message = v8::String::new(scope, &e.to_string()).unwrap();
            let error = v8::Exception::type_error(scope, message);
            scope.throw_exception(error);
            return;
          }
        };

        napi_module_register::MODULE.with(|cell| {
          let slot = *cell.borrow();
          match slot {
            Some(nm) => {
              let nm = unsafe { &*nm };
              assert_eq!(nm.nm_version, 1);
              let exports = unsafe {
                (nm.nm_register_func)(env_ptr, std::mem::transmute(exports))
              };

              println!("{:?}", nm);
              let exports: v8::Local<v8::Value> =
                unsafe { std::mem::transmute(exports) };
              rv.set(exports);
            }
            None => {
              // Initializer callback.
              let init = unsafe {
                library
                  .get::<unsafe extern "C" fn(
                    env: napi_env,
                    exports: napi_value,
                  ) -> napi_value>(b"napi_register_module_v1")
                  .expect("napi_register_module_v1 not found")
              };

              unsafe { init(env_ptr, std::mem::transmute(exports)) };
              rv.set(exports.into());
            }
          }
        });

        std::mem::forget(library);
      },
    )
    .build(inner_scope)
    .unwrap();

    let dlopen_name = v8::String::new(inner_scope, "dlopen").unwrap();

    global
      .set(inner_scope, dlopen_name.into(), dlopen_func.into())
      .unwrap();
  }

  let filename = std::env::args()
    .nth(1)
    .unwrap_or(String::from("./test/example.js"));
  let source_code = std::fs::read_to_string(&filename).unwrap();

  runtime
    .execute_script("core.js", include_str!("core.js"))
    .unwrap();

  match runtime.execute_script(&filename, &source_code) {
    Ok(_) => {}
    Err(e) => {
      eprintln!("{}", e);
      std::process::exit(1);
    }
  }
}
