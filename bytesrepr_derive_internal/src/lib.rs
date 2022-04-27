use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemStruct;

pub fn derive_struct(input: ItemStruct) -> syn::Result<TokenStream2> {
    // TODO[RC]: Should we only support structs with named fields?
    let name = &input.ident;
    let text = format!("Hello, Bytesrepr derive for {}!", name);
    let generated = quote!(
        let x = #text;
        println!("Generated: ## {} ##", x);
    );
    Ok(generated)
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::ItemStruct;

    use crate::derive_struct;

    #[test]
    fn struct_simple() {
        let item_struct: ItemStruct = syn::parse2(quote! {
            struct Simple {
                x: u16,
            }
        })
        .expect("parse test struct");

        let expected = quote!(());
        let actual = derive_struct(item_struct).expect("derive_struct() failed");

        assert_eq!(expected.to_string(), actual.to_string());
    }
}
