use proc_macro::TokenStream;
use syn::ItemStruct;

use bytesrepr_derive_internal::derive_struct;

#[proc_macro_derive(BytesreprSerialize)]
pub fn bytesrepr_serialize(input: TokenStream) -> TokenStream {
    let generated = match syn::parse::<ItemStruct>(input.clone()) {
        Ok(input) => derive_struct(input),
        Err(_) => panic!("`BytesreprSerialize` is only supported for `struct`s"),
    };
    TokenStream::from(match generated {
        Ok(res) => res,
        Err(err) => err.to_compile_error(),
    })
}
