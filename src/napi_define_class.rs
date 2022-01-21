use crate::callback_info::CallbackInfo;
use crate::env::Env;
use crate::ffi::*;
use deno_core::v8;

#[no_mangle]
pub unsafe extern "C" fn napi_define_class(
  env: napi_env,
  utf8name: *const c_char,
  length: usize,
  constructor: napi_callback,
  callback_data: *mut c_void,
  property_count: usize,
  properties: *const napi_property_descriptor,
  result: *mut napi_value,
) -> napi_status {
  let mut env = &mut *(env as *mut Env);

  let constructor =
    v8::External::new(env.scope, std::mem::transmute(constructor));

  let tpl = v8::FunctionTemplate::builder(
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

      // TODO(@littledivy): callback_data
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
  .data(constructor.into())
  .build(env.scope);

  let name_string = std::ffi::CStr::from_ptr(utf8name).to_str().unwrap();
  let name = v8::String::new(env.scope, name_string).unwrap();
  tpl.set_class_name(name);

  let napi_properties = std::slice::from_raw_parts(properties, property_count);
  for p in napi_properties {
    let name = CStr::from_ptr(p.utf8name).to_str().unwrap();
    let name = v8::String::new(env.scope, name).unwrap();

    if !(p.method as *const c_void).is_null() {
      let method_ptr =
        v8::External::new(env.scope, std::mem::transmute(p.method));

      let function = v8::FunctionTemplate::builder(
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
      .build(env.scope);

      let proto = tpl.prototype_template(env.scope);
      proto.set(name.into(), function.into());
    } else if !(p.getter as *const c_void).is_null()
      || !(p.setter as *const c_void).is_null()
    {
      let getter_ptr =
        v8::External::new(env.scope, std::mem::transmute(p.getter));
      let setter_ptr =
        v8::External::new(env.scope, std::mem::transmute(p.setter));

      let getter = if getter_ptr.is_null() {
        None
      } else {
        Some(
          v8::FunctionTemplate::builder(
            |handle_scope: &mut v8::HandleScope,
             args: v8::FunctionCallbackArguments,
             mut rv: v8::ReturnValue| {
              let data = args.data().unwrap();
              let getter_ptr =
                v8::Local::<v8::External>::try_from(data).unwrap();
              let getter: napi_callback =
                std::mem::transmute(getter_ptr.value());

              let context = v8::Context::new(handle_scope);
              let scope = &mut v8::ContextScope::new(handle_scope, context);

              let mut env = Env { scope };
              let env_ptr = &mut env as *mut _ as *mut c_void;

              let mut info = CallbackInfo {
                env: env_ptr,
                cb: getter,
                cb_info: ptr::null_mut(),
                args: &args as *const _ as *const c_void,
              };

              let info_ptr = &mut info as *mut _ as *mut c_void;

              let value = unsafe { getter(env_ptr, info_ptr) };
              let value = std::mem::transmute(value);

              rv.set(value);
            },
          )
          .data(getter_ptr.into())
          .build(env.scope),
        )
      };

      let setter = if setter_ptr.is_null() {
        None
      } else {
        Some(
          v8::FunctionTemplate::builder(
            |handle_scope: &mut v8::HandleScope,
             args: v8::FunctionCallbackArguments,
             mut rv: v8::ReturnValue| {
              let data = args.data().unwrap();
              let setter_ptr =
                v8::Local::<v8::External>::try_from(data).unwrap();
              let setter: napi_callback =
                std::mem::transmute(setter_ptr.value());

              let context = v8::Context::new(handle_scope);
              let scope = &mut v8::ContextScope::new(handle_scope, context);

              let mut env = Env { scope };
              let env_ptr = &mut env as *mut _ as *mut c_void;

              let mut info = CallbackInfo {
                env: env_ptr,
                cb: setter,
                cb_info: ptr::null_mut(),
                args: &args as *const _ as *const c_void,
              };

              let info_ptr = &mut info as *mut _ as *mut c_void;

              let value = unsafe { setter(env_ptr, info_ptr) };
              let value = std::mem::transmute(value);

              rv.set(value);
            },
          )
          .data(setter_ptr.into())
          .build(env.scope),
        )
      };

      let proto = tpl.prototype_template(env.scope);

      let base_name = CStr::from_ptr(p.utf8name).to_str().unwrap();
      let getter_name = v8::String::new(env.scope, format!("get_{}", base_name).as_str()).unwrap();
      let setter_name = v8::String::new(env.scope, format!("set_{}", base_name).as_str()).unwrap();

      // TODO: use set_accessor & set_accessor_with_setter
      match (getter, setter) {
        (Some(getter), None) => {
          proto.set(getter_name.into(), getter.into());
        },
        (Some(getter), Some(setter)) => {
          proto.set(getter_name.into(), getter.into());
          proto.set(setter_name.into(), setter.into());
        },
        (None, Some(setter)) => {
          proto.set(setter_name.into(), setter.into());
        },
        (None, None) => {},
      }
    } else {
      let proto = tpl.prototype_template(env.scope);
      proto.set(name.into(), std::mem::transmute(p.value));
    }
  }

  let value: v8::Local<v8::Value> = tpl.get_function(env.scope).unwrap().into();
  *result = std::mem::transmute(value);

  napi_ok
}
