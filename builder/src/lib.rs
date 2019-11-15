extern crate proc_macro;

use proc_macro::TokenStream;
// use proc_macro2::{Ident, Span};
use quote::{quote, format_ident, quote_spanned, ToTokens};
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // dbg!(&input);
    let name = input.ident;

    let builder_name = format_ident!("{}Builder", name);

    let fields = match input.data {
        Data::Struct(ref obj) => {
            match obj.fields {
                Fields::Named(ref fields) => {
                    fields.named.iter()
                }
                _ => unimplemented!(),
            }
        },
        Data::Enum(_) => unimplemented!(),
        Data::Union(_) => unimplemented!()
    };
    let names = fields.clone().map(|f| &f.ident).collect();
    let tys = fields.map(|f| &f.ty).collect();

    let expanded = quote! {
        pub struct #builder_name {
            #(#names: Option<#tys>),*
        }

        
    };


    TokenStream::from(expanded)
}
