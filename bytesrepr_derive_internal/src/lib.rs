use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{FieldsNamed, Ident, ItemStruct};

fn to_field_names<'a>(fields: &'a FieldsNamed) -> impl Iterator<Item = &'a Ident> {
    // TODO: Support `skip` attribute here.
    fields
        .named
        .iter()
        .filter_map(|field| field.ident.as_ref().or(None))
}

fn generate_serialization_code(input: &ItemStruct) -> syn::Result<(TokenStream2, TokenStream2)> {
    let mut fields_serialization_code = quote!();
    let mut serialized_len_code = quote!();
    match &input.fields {
        syn::Fields::Named(fields) => {
            let field_names: Vec<_> = to_field_names(fields).collect();
            serialized_len_code.extend(quote!(
                #(self.#field_names.serialized_length())+*
            ));
            fields_serialization_code.extend(quote!(
                #(buffer.extend(self.#field_names.to_bytes()?);)*
            ));
        }
        syn::Fields::Unnamed(_) => todo!(),
        syn::Fields::Unit => todo!(),
    }
    Ok((fields_serialization_code, serialized_len_code))
}

pub fn derive_struct(input: ItemStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let (fields_serialization_code, serialized_len_code) = generate_serialization_code(&input)?;
    let generated = quote!(
        impl ToBytes for #struct_name {
            fn to_bytes(&self) -> Result<Vec<u8>, casper_types::bytesrepr::Error> {
                let mut buffer = casper_types::bytesrepr::allocate_buffer(self)?;
                #fields_serialization_code
                Ok(buffer)
            }

            fn serialized_length(&self) -> usize {
                #serialized_len_code
            }
        }
    );
    Ok(generated)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use quote::quote;
    use syn::ItemStruct;

    use crate::{derive_struct, to_field_names};

    #[test]
    fn field_names() {
        let item_struct: ItemStruct = syn::parse2(quote! {
            struct Simple {
                unsigned_int: u16,
                int: i64,
                float: f64,
                string: String,
            }
        })
        .expect("parse test struct");

        let expected = vec!["unsigned_int", "int", "float", "string"];
        if let syn::Fields::Named(fields_named) = item_struct.fields {
            let actual: Vec<_> = to_field_names(&fields_named)
                .map(|ident| ident.to_string())
                .collect();
            assert_eq!(expected, actual);
        } else {
            panic!("unexpected fields");
        }

        let item_struct: ItemStruct = syn::parse2(quote! {
            struct SingleField {
                unsigned_int: u16,
            }
        })
        .expect("parse test struct");

        let expected = vec!["unsigned_int"];
        if let syn::Fields::Named(fields_named) = item_struct.fields {
            let actual: Vec<_> = to_field_names(&fields_named)
                .map(|ident| ident.to_string())
                .collect();
            assert_eq!(expected, actual);
        } else {
            panic!("unexpected fields");
        }

        let item_struct: ItemStruct = syn::parse2(quote! {
            struct NoFields {
            }
        })
        .expect("parse test struct");

        let expected: Vec<String> = vec![];
        if let syn::Fields::Named(fields_named) = item_struct.fields {
            let actual: Vec<_> = to_field_names(&fields_named)
                .map(|ident| ident.to_string())
                .collect();
            assert_eq!(expected, actual);
        } else {
            panic!("unexpected fields");
        }
    }

    #[test]
    fn struct_simple() {
        let item_struct: ItemStruct = syn::parse2(quote! {
            struct Simple {
                unsigned_int: u16,
                int: i64,
                float: f64,
                string: String,
            }
        })
        .expect("parse test struct");

        let expected = quote!(
            impl ToBytes for Simple {
                fn to_bytes(&self) -> Result<Vec<u8>, casper_types::bytesrepr::Error> {
                    let mut buffer = casper_types::bytesrepr::allocate_buffer(self)?;
                    buffer.extend(self.unsigned_int.to_bytes()?);
                    buffer.extend(self.int.to_bytes()?);
                    buffer.extend(self.float.to_bytes()?);
                    buffer.extend(self.string.to_bytes()?);
                    Ok(buffer)
                }
                fn serialized_length(&self) -> usize {
                    self.unsigned_int.serialized_length()
                        + self.int.serialized_length()
                        + self.float.serialized_length()
                        + self.string.serialized_length()
                }
            }
        );
        let actual = derive_struct(item_struct).expect("derive_struct() failed");

        assert_eq!(expected.to_string(), actual.to_string());
    }
}
