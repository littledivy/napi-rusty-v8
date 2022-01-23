use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn napi_sym(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let func = syn::parse::<syn::ItemFn>(item).expect("expected a function");

  let name = &func.sig.ident;
  let block = &func.block;
  // TODO(@littledivy): make first argument &'a mut env::Env?
  let inputs = &func.sig.inputs;
  let output = &func.sig.output;
  let ret_ty = match output {
    syn::ReturnType::Default => panic!("expected a return type"),
    syn::ReturnType::Type(_, ty) => quote! { #ty },
  };
  TokenStream::from(quote! {
      #[no_mangle]
      pub unsafe extern "C" fn #name(#inputs) -> napi_status {
         let result: #ret_ty = #block;
         // TODO: convert error to napi_status
         result.unwrap();
         napi_ok
      }
  })
}
