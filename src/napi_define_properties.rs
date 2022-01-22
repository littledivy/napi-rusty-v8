use crate::callback_info::CallbackInfo;
use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_define_properties(
  env: napi_env,
  obj: napi_value,
  property_count: usize,
  properties: *const napi_property_descriptor,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let object: v8::Local<v8::Object> = std::mem::transmute(obj);
  let properties = std::slice::from_raw_parts(properties, property_count);

  for property in properties {
    let name = CStr::from_ptr(property.utf8name).to_str().unwrap();

    let name = v8::String::new(env.scope, name).unwrap();

    let method_ptr: *mut c_void = std::mem::transmute(property.method);

    if !method_ptr.is_null() {
      let method_ptr = v8::External::new(env.scope, method_ptr);

      let function = v8::Function::builder(
        |handle_scope: &mut v8::HandleScope,
         args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
          let data = args.data().unwrap();
          let method_ptr = v8::Local::<v8::External>::try_from(data).unwrap();
          let method: napi_callback = std::mem::transmute(method_ptr.value());

          let context = v8::Context::new(handle_scope);
          let scope = &mut v8::ContextScope::new(handle_scope, context);

          let mut env = Env { scope };
          let env_ptr = &mut env as *mut _ as *mut c_void;

          let mut info = CallbackInfo {
            env: env_ptr,
            cb: method,
            cb_info: ptr::null_mut(),
            args: &args as *const _ as *const c_void,
          };

          let info_ptr = &mut info as *mut _ as *mut c_void;

          let value = unsafe { method(env_ptr, info_ptr) };
          let value = std::mem::transmute(value);

          rv.set(value);
        },
      )
      .data(method_ptr.into())
      .build(env.scope)
      .unwrap();

      object.set(env.scope, name.into(), function.into()).unwrap();
    }
  }

  napi_ok
}
