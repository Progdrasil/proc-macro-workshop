extern crate proc_macro;

use proc_macro::TokenStream;
// use proc_macro2::{Ident, Span};
use quote::{quote, format_ident};
use syn::{
    Attribute,
    parse_macro_input,
    DeriveInput,
    Data,
    Fields,
    Field,
    Lit,
    Ident,
    Meta,
    NestedMeta,
    Path,
    PathArguments,
    GenericArgument,
    Type,
};

#[derive(Debug)]
struct BuilderData<'ast> {
    ident: &'ast Option<Ident>,
    ty: &'ast Type,
    is_optional: bool,
    attributes: Vec<BuilderAttribute>,
}

#[derive(Debug)]
enum BuilderAttribute {
    Each(String),
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let builder_name = format_ident!("{}Builder", name);

    let mut data = vec![];

    match input.data {
        Data::Struct(ref obj) => {
            match obj.fields {
                Fields::Named(ref fields) => {
                    for field in &fields.named {
                        data.push(get_builder_data(&field));
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

fn get_builder_data(field: &Field) -> BuilderData {
    let mut is_optional = false;

    if let Type::Path(path) = &field.ty {
        for seg in &path.path.segments {
            if seg.ident == "Option"{
                is_optional = true;
            }
        }
    }

    let mut attributes = vec![];
    if !field.attrs.is_empty() {
        for attr in &field.attrs {
            attributes.append(&mut parse_builder_attr(attr));
        }
    }

    BuilderData {
        ident: &field.ident,
        ty: &field.ty,
        is_optional,
        attributes,
    }
}

fn parse_builder_attr(attr: &Attribute) -> Vec<BuilderAttribute> {
    let meta = match attr.parse_meta() {
        Ok(m) => m,
        Err(e) => panic!(e),
    };

    let mut attrs = vec![];
    match meta {
        Meta::List(ml) => {
            if is_builder_attr(&ml.path) {
                for nested in ml.nested {
                    attrs.append(&mut nested_builder_attr(&nested));
                }
            }
        }
        _ => (),
    };

    attrs
}

fn nested_builder_attr(nm: &NestedMeta) -> Vec<BuilderAttribute> {
    let mut attrs = vec![];

    match nm {
        NestedMeta::Meta(meta) => {
            match meta {
                Meta::Path(_) => unimplemented!(),
                Meta::List(ml) => {
                    // deal with path somehow
                    for nested in &ml.nested {
                        attrs.append(&mut nested_builder_attr(&nested));
                    }
                },
                Meta::NameValue(mnv) => {
                    // Doesnt make sense for a named value to have multiple paths
                    if mnv.path.segments[0].ident == "each" {
                         if let Lit::Str(val) = &mnv.lit {
                            attrs.push(BuilderAttribute::Each(val.value()));
                         }
                    } else { unimplemented!(); }
                }
            }
        },
        NestedMeta::Lit(_) => unimplemented!(),
    };

    attrs
}

fn is_builder_attr(path: &Path) -> bool {
    let mut is_builder = false;

    for seg in &path.segments {
        if seg.ident == "builder" {
            is_builder = true;
        }
    }

    is_builder
}


fn builder_def(data: &Vec<BuilderData>, builder_name: &Ident) -> proc_macro2::TokenStream {
    let properties = data.iter().map(|d| builder_properties(d));

    quote!{
        pub struct #builder_name {
            #(#properties),* //
        }
    }
}

fn builder_properties(data: &BuilderData) -> proc_macro2::TokenStream {
    let ty = data.ty;
    let name = data.ident;
    let newty = if data.is_optional {
        quote!{#ty}
    } else {
        quote!{Option<#ty>}
    };

    quote!{#name: #newty}
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

    let ty = if data.is_optional {
        if let Type::Path(path) = data.ty {
            if let PathArguments::AngleBracketed(arg) = &path.path.segments[0].arguments {
                if let GenericArgument::Type(typ) = &arg.args[0] {
                    typ
                } else {unimplemented!();}
            } else {unimplemented!();}
        } else {unimplemented!();}
    } else {
        data.ty
    };

    quote!{
        pub fn #name(&mut self, #name: #ty) -> &mut Self {
            self.#name = Some(#name);
            self
        }
    }
}

fn builder_build(data: &Vec<BuilderData>, name: &Ident) -> proc_macro2::TokenStream {
    let extraction = data.iter().map(|d| builder_build_verification(d));
    let names = data.iter().map(|d| &d.ident);

    quote!{
        pub fn build(&mut self) -> Result<#name, Box<dyn std::error::Error>> {
            #(#extraction)*

            Ok(#name {
                #(#names),*
            })
        }
    }
}

fn builder_build_verification(data: &BuilderData) -> proc_macro2::TokenStream {
    let name = data.ident;

    let message = if let Some(name) = data.ident {
        name.to_string() + " has not been added"
    } else {
        "".into()
    };

    if data.is_optional {
        quote!{let #name = self.#name.to_owned();}
    } else {
        quote!{let #name = if let Some(field) = &self.#name {field.to_owned()} else {return Err(#message.into())};}
    }
}