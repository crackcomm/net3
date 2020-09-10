mod impls;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn rpc(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::ItemTrait);
    self::impls::make_rpc(item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
