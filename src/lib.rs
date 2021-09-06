extern crate proc_macro;
use crate::proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    parenthesized, parse, parse_macro_input, Attribute, Data, DeriveInput, Error, Field, Fields,
    Ident, Index,
};

#[proc_macro_derive(Diff, attributes(diff))]
pub fn diff_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_attr = parse_struct_attributes(&input.attrs);

    match input.data {
        Data::Struct(data_struct) => {
            let name = &input.ident;
            let diff_name = &struct_attr.name.unwrap_or(format_ident!("{}Diff", name));
            let attr = &struct_attr.attrs.0;

            match &data_struct.fields {
                Fields::Named(fields) => derive_named(attr, name, diff_name, &fields.named),
                Fields::Unnamed(fields) => derive_unnamed(attr, name, diff_name, &fields.unnamed),
                Fields::Unit => derive_unit(name),
            }
        }
        _ => todo!(),
    }
}

fn derive_named(
    attr: &[Attribute],
    name: &Ident,
    diff_name: &Ident,
    fields: &Punctuated<Field, Comma>,
) -> TokenStream {
    let names = fields.iter().map(|field| &field.ident).collect::<Vec<_>>();
    let types = fields.iter().map(|field| &field.ty).collect::<Vec<_>>();
    (quote! {
        #(#attr)*
        pub struct #diff_name {
            #(pub #names: <#types as Diff>::Repr),*
        }

        impl Diff for #name {
            type Repr = #diff_name;

            fn diff(&self, other: &Self) -> Self::Repr {
                #diff_name {
                    #(#names: self.#names.diff(&other.#names)),*
                }
            }

            fn apply(&mut self, diff: &Self::Repr) {
                #(self.#names.apply(&diff.#names);)*
            }

            fn identity() -> Self {
                Self {
                    #(#names: <#types as Diff>::identity()),*
                }
            }
        }
    })
    .into()
}

fn derive_unnamed(
    attr: &[Attribute],
    name: &Ident,
    diff_name: &Ident,
    fields: &Punctuated<Field, Comma>,
) -> TokenStream {
    let (numbers, types): (Vec<_>, Vec<_>) = fields
        .iter()
        .map(|field| &field.ty)
        .enumerate()
        .map(|(a, b)| (Index::from(a), b))
        .unzip();
    (quote! {
        #(#attr)*
        pub struct #diff_name (
            #(pub <#types as Diff>::Repr),*
        );

        impl Diff for #name {
            type Repr = #diff_name;

            fn diff(&self, other: &Self) -> Self::Repr {
                #diff_name (
                    #(self.#numbers.diff(&other.#numbers))*
                )
            }

            fn apply(&mut self, diff: &Self::Repr) {
                #(self.#numbers.apply(&diff.#numbers);)*
            }

            fn identity() -> Self {
                Self (
                    #(<#types as Diff>::identity()),*
                )
            }
        }
    })
    .into()
}

#[derive(Default)]
struct StructAttributes {
    name: Option<Ident>,
    visible: bool,
    attrs: OuterAttributes,
}

/// A named attribute with unspecified tokens inside parentheses
struct ParenAttr {
    name: Ident,
    tokens: TokenStream,
}

impl Parse for ParenAttr {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let name = input.parse()?;
        let content;
        parenthesized!(content in input);
        Ok(ParenAttr {
            name,
            tokens: content.parse::<proc_macro2::TokenStream>()?.into(),
        })
    }
}

#[derive(Default)]
struct OuterAttributes(Vec<Attribute>);

impl Parse for OuterAttributes {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        Ok(Self(input.call(Attribute::parse_outer)?))
    }
}

fn parse_struct_attributes(attrs: &[Attribute]) -> StructAttributes {
    let mut struct_attrs = StructAttributes::default();
    attrs
        .iter()
        .filter(|a| a.path.is_ident("diff"))
        .for_each(|a| {
            let attr_named: ParenAttr = a.parse_args().unwrap();
            let name = attr_named.name.to_string();
            match name.as_ref() {
                "attr" => {
                    struct_attrs.attrs = parse(attr_named.tokens).unwrap();
                }
                _ => panic!(
                    "Unexpected name for diff attribute '{}'. Possible names: 'attr'",
                    name
                ),
            }
        });
    struct_attrs
}

fn derive_unit(
    name: &Ident,
) -> TokenStream {
    (quote! {
        impl Diff for #name {
            type Repr = ();

            fn diff(&self, other: &Self) -> Self::Repr {
                ()
            }

            fn apply(&mut self, diff: &Self::Repr) {
                ()
            }

            fn identity() -> Self {
                Self
            }
        }
    })
    .into()
}

// fn parse_field_attributes(attrs: &[Attribute]) -> FieldAttributes {
//     let field_attrs = FieldAttributes::default();
//     // iter
//     field_attrs
// }
