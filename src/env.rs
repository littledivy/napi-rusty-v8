use deno_core::v8;

pub struct Env<'a, 'b, 'c> {
  pub scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
}
