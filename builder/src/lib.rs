extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input, export::Span, Data, Fields, FieldsNamed};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
	let ast = parse_macro_input!(input as DeriveInput);
	let vis = &ast.vis;
	let id = &ast.ident;
	let builder_id = Ident::new(&format!("{}Builder", id), Span::call_site());

	let (builder_init, builder_fields, builder_setters) = match ast.data {
		Data::Struct(s)=> {
			match s.fields {
				Fields::Named(f) => {
					(
						create_builder_init(&f), 
						create_builder_fields(&f),
						create_builder_setters(&f)
					)
				},
				_ => unimplemented!()
			}
		},
		_ => unimplemented!(),
	};

	let tokens = quote!{
		#vis struct #builder_id {
			#builder_fields
		}

		impl #id {
			#vis fn builder() -> #builder_id {
				#builder_id {
					#builder_init
				}
			}
		}

		impl #builder_id {
			#builder_setters
		}
	};

	tokens.into()	
}

fn create_builder_fields(fields: &FieldsNamed) -> proc_macro2::TokenStream {
	let fds = fields.named.iter().map(|f| {
		let id = &f.ident;
		let ty = &f.ty;

		quote!{
			#id: ::std::option::Option<#ty>
		}
	});

	quote!{ #(#fds),* }
}

fn create_builder_init(fields: &FieldsNamed) -> proc_macro2::TokenStream {
	let inits = fields.named.iter().map(|f| {
		let id = &f.ident;
		
		quote!{
			#id: ::std::option::Option::None
		}
	});
	quote!{ #(#inits),*}
}

fn create_builder_setters(fields: &FieldsNamed) -> proc_macro2::TokenStream {
	let setters = fields.named.iter().map(|f| {
		let id = &f.ident;
		let ty = &f.ty;

		quote!{
			fn #id(&mut self, #id: #ty) -> &mut Self {
				self.#id = ::std::option::Option::Some(#id);
				self
			}
		}
	});
	quote!{ #(#setters)* }
}