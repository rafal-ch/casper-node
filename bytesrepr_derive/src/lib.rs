#![no_std]

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::crate_name;
use quote::quote;
use syn::{Ident, ItemStruct};

use bytesrepr_derive_internal::{deserialize_struct, serialize_struct};

fn detect_crate_name() -> Result<proc_macro2::TokenStream, TokenStream> {
    let crate_name = match crate_name("casper-types") {
        Ok(name) => match name {
            proc_macro_crate::FoundCrate::Itself => {
                let ident = Ident::new("crate", Span::call_site());
                quote!(#ident)
            }
            proc_macro_crate::FoundCrate::Name(name) => {
                let ident = Ident::new(&name, Span::call_site());
                quote!(#ident)
            }
        },
        Err(_) => {
            let error = quote!(compile_error!("crate name not found"));
            return Err(TokenStream::from(error));
        }
    };
    Ok(crate_name)
}

#[cfg(feature = "serialize")]
#[proc_macro_derive(BytesreprSerialize)]
pub fn bytesrepr_serialize(input: TokenStream) -> TokenStream {
    let crate_name = match detect_crate_name() {
        Ok(value) => value,
        Err(value) => return value,
    };

    let generated = match syn::parse::<ItemStruct>(input.clone()) {
        Ok(input) => serialize_struct(&input, &crate_name),
        Err(_) => panic!("`BytesreprSerialize` is only supported for `struct`s"),
    };

    TokenStream::from(match generated {
        Ok(res) => res,
        Err(err) => err.to_compile_error(),
    })
}

#[cfg(feature = "deserialize")]
#[proc_macro_derive(BytesreprDeserialize)]
pub fn bytesrepr_deserialize(input: TokenStream) -> TokenStream {
    let crate_name = match detect_crate_name() {
        Ok(value) => value,
        Err(value) => return value,
    };

    let generated = match syn::parse::<ItemStruct>(input.clone()) {
        Ok(input) => deserialize_struct(&input, &crate_name),
        Err(_) => panic!("`BytesreprDeserialize` is only supported for `struct`s"),
    };

    TokenStream::from(match generated {
        Ok(res) => res,
        Err(err) => err.to_compile_error(),
    })
}
