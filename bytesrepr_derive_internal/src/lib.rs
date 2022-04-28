use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{FieldsNamed, Ident, ItemStruct, Type};

fn to_field_names<'a>(fields: &'a FieldsNamed) -> impl Iterator<Item = &'a Ident> {
    // TODO: Support `skip` attribute here.
    fields
        .named
        .iter()
        .filter_map(|field| field.ident.as_ref().or(None))
}

// TODO: Merge with `to_field_names`
fn to_field_types<'a>(fields: &FieldsNamed) -> impl Iterator<Item = Type> + '_ {
    // TODO: Support `skip` attribute here.
    fields.named.iter().map(|field| field.ty.clone())
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

fn generate_deserialization_code(input: &ItemStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let mut fields_deserialization_code = quote!();
    let mut fields_list_code = quote!();
    match &input.fields {
        syn::Fields::Named(fields) => {
            let field_names: Vec<_> = to_field_names(fields).collect();
            let field_types: Vec<_> = to_field_types(fields).collect();
            fields_deserialization_code.extend(quote!(
                #(let (#field_names, remainder) = #field_types::from_bytes(remainder)?;)*
            ));
            fields_list_code.extend(quote!(
                #(#field_names),*
            ));
        }
        syn::Fields::Unnamed(_) => todo!(),
        syn::Fields::Unit => todo!(),
    }
    fields_deserialization_code.extend(quote!(
        Ok((#struct_name { #fields_list_code }, remainder))
    ));
    Ok(fields_deserialization_code)
}

pub fn serialize_struct(input: &ItemStruct) -> syn::Result<TokenStream2> {
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

pub fn deserialize_struct(input: &ItemStruct) -> syn::Result<TokenStream2> {
    let fields_deserialization_code = generate_deserialization_code(&input)?;
    let generated = quote!(
        impl FromBytes for Simple {
            fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), casper_types::bytesrepr::Error> {
                let remainder = bytes;
                #fields_deserialization_code
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

    use crate::{deserialize_struct, serialize_struct, to_field_names};

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

        let expected_serialize = quote!(
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

        let expected_deserialize = quote!(
            impl FromBytes for Simple {
                fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), casper_types::bytesrepr::Error> {
                    let remainder = bytes;
                    let (unsigned_int, remainder) = u16::from_bytes(remainder)?;
                    let (int, remainder) = i64::from_bytes(remainder)?;
                    let (float, remainder) = f64::from_bytes(remainder)?;
                    let (string, remainder) = String::from_bytes(remainder)?;
                    Ok((Simple {
                        unsigned_int,
                        int,
                        float,
                        string
                    }, remainder))
                }
            }
        );

        let actual_serialize = serialize_struct(&item_struct).expect("serialize_struct() failed");
        assert_eq!(expected_serialize.to_string(), actual_serialize.to_string());

        let actual_deserialize =
            deserialize_struct(&item_struct).expect("deserialize_struct() failed");
        assert_eq!(
            expected_deserialize.to_string(),
            actual_deserialize.to_string()
        );
    }
}
