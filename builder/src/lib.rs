extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input, export::Span, Data, Fields, FieldsNamed, Type};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
	let ast = parse_macro_input!(input as DeriveInput);
	let vis = &ast.vis;
	let id = &ast.ident;
	let builder_id = Ident::new(&format!("{}Builder", id), Span::call_site());

	let (builder_init, builder_fields, builder_setters, builder_build) = match ast.data {
		Data::Struct(s)=> {
			match s.fields {
				Fields::Named(f) => {
					let optionals = get_optionals(&f);
					let seperates = get_seperates(&f);
					(
						create_builder_init(&f, &seperates),
						create_builder_fields(&f, &optionals, &seperates),
						create_builder_setters(&f, &optionals, &seperates),
						create_builder_build(&id, &f, &optionals, &seperates),
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

			#builder_build
		}
	};

	tokens.into()	
}

fn get_seperates(fields: &FieldsNamed) -> Vec<&syn::Field> {
	fields.named.iter()
		.filter(|f| f.attrs.len() > 0 && f.attrs.iter().any(|attr| if attr.path.segments[0].ident == "builder" {
			if let Ok(parsed_meta) = attr.parse_meta() {
				let attr_ident = match parsed_meta {
					syn::Meta::List(ref list) => match list.nested[0] {
						syn::NestedMeta::Meta(syn::Meta::NameValue(ref named)) => &named.path.segments[0].ident,
						_ => unreachable!()
					},
					_ => unreachable!()
				};
				attr_ident == "each"
			} else {
				false
			}
		} else {
			false
		}))
		.collect()
}

fn get_optionals(fields: &FieldsNamed) -> Vec<&syn::Field> {
	fields.named.iter()
		.filter(|f| if let Type::Path(ref ty_path) = f.ty {
			ty_path.path.segments[0].ident == "Option"
		} else {
			false
		})
		.collect()
}

fn create_builder_fields(fields: &FieldsNamed, optionals: &[&syn::Field], seperates: &[&syn::Field]) -> proc_macro2::TokenStream {
	let fds = fields.named.iter().map(|f| {
		let id = &f.ident;
		let ty = &f.ty;

		if optionals.contains(&f) || seperates.contains(&f) {
			quote!{
				#id: #ty
			}
		} else {
			quote!{
				#id: ::std::option::Option<#ty>
			}
		}
	});

	quote!{ #(#fds),* }
}

fn create_builder_init(fields: &FieldsNamed, seperates: &[&syn::Field]) -> proc_macro2::TokenStream {
	let inits = fields.named.iter().map(|f| {
		let id = &f.ident;

		if seperates.contains(&f) {
			let ty = &f.ty;
			quote!{
				#id: <#ty>::new()
			}
		} else {
			quote!{
				#id: ::std::option::Option::None
			}
		}
		
	});
	quote!{ #(#inits),*}
}

fn create_builder_setters(fields: &FieldsNamed, optionals: &[&syn::Field], seperates: &[&syn::Field]) -> proc_macro2::TokenStream {
	let setters = fields.named.iter().map(|f| {
		let id = &f.ident;
		let ty = if optionals.contains(&f) || seperates.contains(&f){
			get_inner_ty(f)
		} else {
			&f.ty
		};

		if seperates.contains(&f) {
			let fn_id = extract_name(&f);

			let og = if &fn_id != id.as_ref().unwrap() {
				let wrapped_ty = &f.ty;
				quote!{
					fn #id(&mut self, #id: &mut #wrapped_ty) -> &mut Self {
						self.#id.append(#id);
						self
					}
				}
			} else { quote!{} };

			quote!{
				#og

				fn #fn_id(&mut self, #id: #ty) -> &mut Self {
					self.#id.push(#id);
					self
				}
			}
		} else {
			quote!{
				fn #id(&mut self, #id: #ty) -> &mut Self {
					self.#id = ::std::option::Option::Some(#id);
					self
				}
			}
		}

	});
	quote!{ #(#setters)* }
}

fn create_builder_build(struct_ident: &Ident, fields:&FieldsNamed, optionals: &[&syn::Field], seperates: &[&syn::Field]) -> proc_macro2::TokenStream {
	let requirement_check = fields.named.iter().map(|f| {
		let id = &f.ident;
		let struct_ident_str = struct_ident.to_string();
		if !optionals.contains(&f) && !seperates.contains(&f) {
			quote!{
				if self.#id.is_none() {
					return ::std::result::Result::Err(::std::boxed::Box::from(format!("{} Value not set for field: {:#?}", #struct_ident_str, self.#id)))
				}
			}
		} else { quote!{} }
	});

	let field_acquisition = fields.named.iter().map(|f| {
		let id = &f.ident;

		if optionals.contains(&f) {
			quote!{
				#id: self.#id.take()
			}
		} else if seperates.contains(&f) {
			quote!{
				#id: self.#id.drain(..).collect()
			}
		} else {
			quote!{
				#id: self.#id.take().unwrap()
			}
		}
	});

	quote!(
		fn build(&mut self) -> ::std::result::Result<#struct_ident, ::std::boxed::Box<dyn ::std::error::Error>> {
			#(#requirement_check)*

			::std::result::Result::Ok(#struct_ident {
				#(#field_acquisition),*
			})
		}
	)
}

fn get_inner_ty(field: &syn::Field) -> &Type {
	match field.ty {
		Type::Path(ref ty_p) => match ty_p.path.segments[0].arguments {
			syn::PathArguments::AngleBracketed(ref turbo) => match turbo.args[0] {
				syn::GenericArgument::Type(ref ty) => ty,
				_ => unreachable!(),
			},
			_ => unreachable!(),
		}
		_ => unreachable!(),
	}
}

fn extract_name(field: &syn::Field) -> Ident {
	let attr = field.attrs
		.iter()
		.filter_map(|a| a.parse_meta().ok())
		.nth(0);

	

	let attr_ident = match attr {
		Some(syn::Meta::List(ref list)) => match list.nested[0] {
			syn::NestedMeta::Meta(syn::Meta::NameValue(ref named)) => match &named.lit {
				syn::Lit::Str(lit_str) => lit_str,
				_ => unreachable!()
			},
			_ => unreachable!()
		},
		_ => unreachable!()
	};

	Ident::new(&attr_ident.value(), attr_ident.span())
}