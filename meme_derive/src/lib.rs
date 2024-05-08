extern crate proc_macro;

use core::panic;

use proc_macro::TokenStream;
use quote::{quote, ToTokens, format_ident};
use syn;
use syn::DeriveInput;

#[proc_macro_derive(IObj, attributes(id, data, obj_type, obj_data_type, obj_id_type))]
pub fn iobj_macro_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();

    let name = ast.ident;
    let mut id: Option<syn::Ident> = None; 
    let mut data: Option<syn::Ident> = None; 
    let mut id_t_s = String::new();
    let mut data_t_s = String::new();
    let mut obj_t_s = String::new();

    for root_a in ast.attrs {
        if root_a.path.is_ident("obj_id_type") {
            id_t_s = root_a.tokens.to_string();
            id_t_s.remove(0);
            id_t_s.pop();
        } else if root_a.path.is_ident("obj_data_type") {
            data_t_s = root_a.tokens.to_string();
            data_t_s.remove(0);
            data_t_s.pop();
        } else if root_a.path.is_ident("obj_type") {
            obj_t_s = root_a.tokens.to_string();
            obj_t_s.remove(0);
            obj_t_s.pop();
        }
    }

    let obj_t_val = if obj_t_s.is_empty() {
        quote! {ObjT::Normal}
    } else {
        let val = syn::parse_str::<syn::Expr>(obj_t_s.as_str()).unwrap();
        quote! {#val}
    };

    let gener = ast.generics.clone();
    let where_clu = if let Some(w) = gener.where_clause.clone() {
        let where_ts = w.to_token_stream();
        quote! { #where_ts }
    } else { 
        quote! {}
    };

    if let syn::Data::Struct(s) = ast.data {
        if let syn::Fields::Named(fields) = s.fields {
            for f in fields.named {
                if let Some(attr) = f.attrs.get(0) {
                    if attr.path.is_ident("id") {
                        id = f.ident.clone();
                    } else if attr.path.is_ident("data") {
                        data = f.ident.clone();
                    }
                }
            }
        } else {
            panic!()
        }
    } else {
        panic!()
    }

    let (get_copy_data_vec_body, get_ref_data_vec_body) = if data.is_none() {
        (quote!{ unimplemented!() }, quote!{ unimplemented!() })
    } else {
        let data_ident = data.unwrap();
        (
            quote! {
            self.#data_ident.clone()
            },
            quote! {
                &self.#data_ident
            }
        )
    };

    let id_ident = id.unwrap();
    let id_t_ident = format_ident!("{id_t_s}");
    let data_t_ident = format_ident!("{data_t_s}");
    let gen = quote! {
        impl #gener IObj for #name #gener 
        #where_clu {
            type IdType = #id_t_ident;
            type ValueType = #data_t_ident;
            fn get_id(self: &Self) -> Self::IdType {self.#id_ident.clone()}
            fn get_obj_type(self: &Self) -> ObjType {ObjType::new::<Self>(#obj_t_val)}
            fn get_copy_data_vec(self: &Self) -> Vec<Self::ValueType> { #get_copy_data_vec_body }
            fn get_ref_data_vec(self: &Self) -> &Vec<Self::ValueType> { #get_ref_data_vec_body }
        }
    };
    gen.into()
}
