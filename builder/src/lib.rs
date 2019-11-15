extern crate proc_macro;

use proc_macro::TokenStream;
// use proc_macro2::{Ident, Span};
use quote::{quote, format_ident};
use syn::{parse_macro_input, DeriveInput, Data, Fields, Ident, Type};

struct BuilderData<'ast> {
    ident: &'ast Option<Ident>,
    ty: &'ast Type,
}

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // dbg!(&input);
    let name = input.ident;

    let builder_name = format_ident!("{}Builder", name);

    let mut data = vec![];

    match input.data {
        Data::Struct(ref obj) => {
            match obj.fields {
                Fields::Named(ref fields) => {
                    for field in &fields.named {
                        let ident = &field.ident;
                        data.push(BuilderData {
                            ident,
                            ty: &field.ty
                        });
                    }
                }
                _ => unimplemented!(),
            }
        },
        Data::Enum(_) => unimplemented!(),
        Data::Union(_) => unimplemented!()
    };

    let definition = builder_def(&data, &builder_name);
    let constructor = builder_constructor(&data, &builder_name);
    let methods = impl_builder(&data, &builder_name, &name);

    let expanded = quote! {
        #definition

        impl #name {
            #constructor
        }

        #methods
    };

    TokenStream::from(expanded)
}

fn builder_def(data: &Vec<BuilderData>, builder_name: &Ident) -> proc_macro2::TokenStream {
    let names = data.iter().map(|d| &d.ident);
    let tys = data.iter().map(|d| &d.ty);

    quote!{
        pub struct #builder_name {
            #(#names: Option<#tys>),* //
        }
    }
}

fn builder_constructor(data: &Vec<BuilderData>, builder_name: &Ident) -> proc_macro2::TokenStream {
    let names = data.iter().map(|d| &d.ident);

    quote!{
        pub fn builder() -> #builder_name {
            #builder_name {
                #(#names: None),*
            }
        }
    }
}

fn impl_builder(data: &Vec<BuilderData>, builder_name: &Ident, name: &Ident) -> proc_macro2::TokenStream {
    let methods = data.iter().map(|d| builder_method(d));
    let build = builder_build(data, name);
    quote!{
        impl #builder_name {
            #(#methods)*
            #build
        }
    }
}

fn builder_method(data: &BuilderData) -> proc_macro2::TokenStream {
    let name = data.ident;
    let ty = data.ty;

    quote!{
        pub fn #name(&mut self, #name: #ty) -> &mut Self {
            self.#name = Some(#name);
            self
        }
    }
}

fn builder_build(data: &Vec<BuilderData>, name: &Ident) -> proc_macro2::TokenStream {
    let names = data.iter().map(|d| &d.ident);
    let assing_names = names.clone();
    // let tys = data.iter().map(|d| &d.ty);
    let messages = data.iter().map(|d| {
        if let Some(name) = d.ident {
            name.to_string() + " has not been added"
        } else {
            "".into()
        }
    });

    quote!{
        pub fn build(&mut self) -> Result<#name, Box<dyn std::error::Error>> {
            #(let #names = if let Some(field) = &self.#names {field.to_owned()} else {return Err(#messages.into())};)*

            Ok(#name {
                #(#assing_names),*
            })
        }
    }
}
