#![no_std]

extern crate alloc;
use alloc::vec::Vec;

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

pub fn serialize_struct(
    input: &ItemStruct,
    crate_name: &TokenStream2,
) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let (fields_serialization_code, serialized_len_code) = generate_serialization_code(&input)?;
    let generated = quote!(
        impl ToBytes for #struct_name {
            fn to_bytes(&self) -> Result<Vec<u8>, #crate_name::bytesrepr::Error> {
                let mut buffer = #crate_name::bytesrepr::allocate_buffer(self)?;
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

pub fn deserialize_struct(
    input: &ItemStruct,
    crate_name: &TokenStream2,
) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let fields_deserialization_code = generate_deserialization_code(&input)?;

    // TODO[RC]: This can probably stay as a generic impl in `casper-types` (?)
    let vec_deserialization_code = generate_vector_deserialization_code(&input)?;

    let generated = quote!(
        impl FromBytes for #struct_name {
            fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), #crate_name::bytesrepr::Error> {
                let remainder = bytes;
                #fields_deserialization_code
            }
        }
        #vec_deserialization_code
    );
    Ok(generated)
}

fn generate_deserialization_code(input: &ItemStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let mut fields_deserialization_code = quote!();
    let mut fields_list_code = quote!();
    match &input.fields {
        syn::Fields::Named(fields) => {
            let field_names: Vec<_> = to_field_names(fields).collect();
            let field_types: Vec<_> = to_field_types(fields).map(|ty| match ty {
                // Remove generics part, so that `Vec<u64>` is deserialized
                // as `Vec::from_bytes()` and not `Vec<u64>::from_bytes()`
                Type::Path(path) => path.path.segments.first().unwrap().ident.clone(),
                _ => unimplemented!(),
            }).collect();
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

pub fn generate_vector_deserialization_code(input: &ItemStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    Ok(quote!(
        fn ensure_efficient_serialization<#struct_name>() {
            #[cfg(debug_assertions)]
            debug_assert_ne!(
                any::type_name::<#struct_name>(),
                any::type_name::<u8>(),
                "You should use Bytes newtype wrapper for efficiency"
            );
        }

        fn try_vec_with_capacity<#struct_name>(capacity: usize) -> Result<Vec<#struct_name>, Error> {
            let elem_size = mem::size_of::<#struct_name>();
            let alloc_size = capacity.checked_mul(elem_size).ok_or(Error::OutOfMemory)?;

            let ptr = if alloc_size == 0 {
                NonNull::<#struct_name>::dangling()
            } else {
                let align = mem::align_of::<#struct_name>();
                let layout =
                    Layout::from_size_align(alloc_size, align).map_err(|_| Error::OutOfMemory)?;
                let raw_ptr = unsafe { alloc(layout) };
                let non_null_ptr = NonNull::<u8>::new(raw_ptr).ok_or(Error::OutOfMemory)?;
                non_null_ptr.cast()
            };
            unsafe { Ok(Vec::from_raw_parts(ptr.as_ptr(), 0, capacity)) }
        }

        fn vec_from_vec(bytes: Vec<u8>) -> Result<(Vec<#struct_name>, Vec<u8>), Error> {
            ensure_efficient_serialization::<#struct_name>();

            Vec::<#struct_name>::from_bytes(bytes.as_slice()).map(|(x, remainder)| (x, Vec::from(remainder)))
        }

        impl FromBytes for Vec<#struct_name> {
            fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
                ensure_efficient_serialization::<#struct_name>();

                let (count, mut stream) = u32::from_bytes(bytes)?;

                let mut result = try_vec_with_capacity(count as usize)?;
                for _ in 0..count {
                    let (value, remainder) = #struct_name::from_bytes(stream)?;
                    result.push(value);
                    stream = remainder;
                }

                Ok((result, stream))
            }

            fn from_vec(bytes: Vec<u8>) -> Result<(Self, Vec<u8>), Error> {
                vec_from_vec(bytes)
            }
        }
    ))
}

#[cfg(test)]
mod tests {
    use alloc::{
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    use pretty_assertions::assert_eq;
    use proc_macro2::{Span, TokenStream};
    use quote::{__private::ext::RepToTokensExt, quote};
    use syn::{Ident, ItemStruct};

    use crate::{deserialize_struct, serialize_struct, to_field_names, to_field_types};

    fn casper_types_crate_name() -> TokenStream {
        let ident = Ident::new("casper_types", Span::call_site());
        quote!(#ident)
    }

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
    fn field_types() {
        let item_struct: ItemStruct = syn::parse2(quote! {
            struct Simple {
                unsigned_int: u16,
                int: i64,
                float: f64,
                string: String,
            }
        })
        .expect("parse test struct");

        let expected = vec!["Ident(u16)", "Ident(i64)", "Ident(f64)", "Ident(String)"];
        if let syn::Fields::Named(fields_named) = item_struct.fields {
            let actual: Vec<_> = to_field_types(&fields_named)
                .map(|ty| {
                    format!(
                        "{}",
                        match ty {
                            // TODO[RC]: Calling `.ident` removes the information about generics.
                            // And we should test with generics here.
                            syn::Type::Path(path) => format!(
                                "{:?}",
                                path.path.segments.next().unwrap().first().unwrap().ident
                            ),
                            _ => panic!("unsupported"),
                        }
                    )
                })
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

        let expected = vec!["Ident(u16)"];
        if let syn::Fields::Named(fields_named) = item_struct.fields {
            let actual: Vec<_> = to_field_types(&fields_named)
                .map(|ty| {
                    format!(
                        "{}",
                        match ty {
                            syn::Type::Path(path) => format!(
                                "{:?}",
                                path.path.segments.next().unwrap().first().unwrap().ident
                            ),
                            _ => panic!("unsupported"),
                        }
                    )
                })
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

        let expected = Vec::<String>::new();
        if let syn::Fields::Named(fields_named) = item_struct.fields {
            let actual: Vec<_> = to_field_types(&fields_named)
                .map(|ty| {
                    format!(
                        "{}",
                        match ty {
                            syn::Type::Path(path) => format!(
                                "{:?}",
                                path.path.segments.next().unwrap().first().unwrap().ident
                            ),
                            _ => panic!("unsupported"),
                        }
                    )
                })
                .collect();
            assert_eq!(expected, actual);
        } else {
            panic!("unexpected fields");
        }

        let item_struct: ItemStruct = syn::parse2(quote! {
            struct WithGenerics {
                vec_unsigned_int: Vec<u16>,
                vec_string: Vec<String>,
            }
        })
        .expect("parse test struct");

        let expected = vec!["Ident(Vec)", "Ident(Vec)"];
        if let syn::Fields::Named(fields_named) = item_struct.fields {
            let actual: Vec<_> = to_field_types(&fields_named)
                .map(|ty| {
                    format!(
                        "{}",
                        match ty {
                            syn::Type::Path(path) => format!(
                                "{:?}",
                                path.path.segments.next().unwrap().first().unwrap().ident
                            ),
                            _ => panic!("unsupported"),
                        }
                    )
                })
                .collect();
            assert_eq!(expected, actual);
        } else {
            panic!("unexpected fields");
        }
    }

    #[test]
    fn struct_simple() {
        let crate_name = casper_types_crate_name();

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
                fn to_bytes(&self) -> Result<Vec<u8>, #crate_name::bytesrepr::Error> {
                    let mut buffer = #crate_name::bytesrepr::allocate_buffer(self)?;
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
                    Ok((
                        Simple {
                            unsigned_int,
                            int,
                            float,
                            string
                        },
                        remainder
                    ))
                }
            }
            fn ensure_efficient_serialization<Simple>() {
                #[cfg(debug_assertions)]
                debug_assert_ne!(
                    any::type_name::<Simple>(),
                    any::type_name::<u8>(),
                    "You should use Bytes newtype wrapper for efficiency"
                );
            }
            fn try_vec_with_capacity<Simple>(capacity: usize) -> Result<Vec<Simple>, Error> {
                let elem_size = mem::size_of::<Simple>();
                let alloc_size = capacity.checked_mul(elem_size).ok_or(Error::OutOfMemory)?;
                let ptr = if alloc_size == 0 {
                    NonNull::<Simple>::dangling()
                } else {
                    let align = mem::align_of::<Simple>();
                    let layout = Layout::from_size_align(alloc_size, align).map_err(|_| Error::OutOfMemory)?;
                    let raw_ptr = unsafe { alloc(layout) };
                    let non_null_ptr = NonNull::<u8>::new(raw_ptr).ok_or(Error::OutOfMemory)?;
                    non_null_ptr.cast()
                };
                unsafe { Ok(Vec::from_raw_parts(ptr.as_ptr(), 0, capacity)) }
            }
            fn vec_from_vec(bytes: Vec<u8>) -> Result<(Vec<Simple>, Vec<u8>), Error> {
                ensure_efficient_serialization::<Simple>();
                Vec::<Simple>::from_bytes(bytes.as_slice()).map(|(x, remainder)| (x, Vec::from(remainder)))
            }
            impl FromBytes for Vec<Simple> {
                fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
                    ensure_efficient_serialization::<Simple>();
                    let (count, mut stream) = u32::from_bytes(bytes)?;
                    let mut result = try_vec_with_capacity(count as usize)?;
                    for _ in 0..count {
                        let (value, remainder) = Simple::from_bytes(stream)?;
                        result.push(value);
                        stream = remainder;
                    }
                    Ok((result, stream))
                }
                fn from_vec(bytes: Vec<u8>) -> Result<(Self, Vec<u8>), Error> {
                    vec_from_vec(bytes)
                }
            }
        );

        let actual_serialize = serialize_struct(&item_struct, &casper_types_crate_name())
            .expect("serialize_struct() failed");
        assert_eq!(expected_serialize.to_string(), actual_serialize.to_string());

        let actual_deserialize = deserialize_struct(&item_struct, &casper_types_crate_name())
            .expect("deserialize_struct() failed");
        assert_eq!(
            expected_deserialize.to_string(),
            actual_deserialize.to_string()
        );
    }

    #[test]
    fn struct_with_vector() {
        let crate_name = casper_types_crate_name();

        let item_struct: ItemStruct = syn::parse2(quote! {
                    struct Simple {
                        data_0: Vec<i64>,
        //                data_1: u8,
        //                data_2: Vec<String>,
                    }
                })
        .expect("parse test struct");

        let expected_serialize = quote!();

        let expected_deserialize = quote!();

        // let actual_serialize = serialize_struct(&item_struct, &casper_types_crate_name())
        //     .expect("serialize_struct() failed");
        // assert_eq!(expected_serialize.to_string(), actual_serialize.to_string());

        let actual_deserialize = deserialize_struct(&item_struct, &casper_types_crate_name())
            .expect("deserialize_struct() failed");
        assert_eq!(
            expected_deserialize.to_string(),
            actual_deserialize.to_string()
        );
    }
}
