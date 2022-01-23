use deno_core::v8;

#[derive(Debug)]
pub struct Env<'a, 'b, 'c> {
  pub scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
  // pub test: u32,
}

impl<'a, 'b, 'c> Env<'a, 'b, 'c> {
  pub fn new(scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>) -> Self {
    Self { scope /* , test: 69 */ }
  }

  pub fn with_new_scope(
    &self,
    scope: &'a mut v8::ContextScope<'b, v8::HandleScope<'c>>,
  ) -> Self {
    // println!("old scope: {:?}", self.scope);
    Self { scope, /* test: self.test */ }
  }
}
