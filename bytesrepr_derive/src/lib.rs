use proc_macro::TokenStream;
use syn::ItemStruct;

use bytesrepr_derive_internal::{deserialize_struct, serialize_struct};

#[proc_macro_derive(BytesreprSerialize)]
pub fn bytesrepr_serialize(input: TokenStream) -> TokenStream {
    let generated = match syn::parse::<ItemStruct>(input.clone()) {
        Ok(input) => serialize_struct(&input),
        Err(_) => panic!("`BytesreprSerialize` is only supported for `struct`s"),
    };
    TokenStream::from(match generated {
        Ok(res) => res,
        Err(err) => err.to_compile_error(),
    })
}

#[proc_macro_derive(BytesreprDeserialize)]
pub fn bytesrepr_deserialize(input: TokenStream) -> TokenStream {
    let generated = match syn::parse::<ItemStruct>(input.clone()) {
        Ok(input) => deserialize_struct(&input),
        Err(_) => panic!("`BytesreprDeserialize` is only supported for `struct`s"),
    };
    TokenStream::from(match generated {
        Ok(res) => res,
        Err(err) => err.to_compile_error(),
    })
}
