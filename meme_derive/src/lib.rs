extern crate proc_macro;

use core::panic;

use proc_macro::TokenStream;
use quote::{quote, ToTokens, format_ident};
use syn;
use syn::DeriveInput;

#[proc_macro_derive(IObj, attributes(id, data))]
pub fn iobj_macro_derive(input: TokenStream) -> TokenStream {
    // 基于 input 构建 AST 语法树
    let ast: DeriveInput = syn::parse(input).unwrap();

    let name = ast.ident;
    let mut id: Option<syn::Ident> = None; 
    let mut data: Option<syn::Ident> = None; 
    let mut id_t_s = String::new();
    let mut data_t_s = String::new();

    let gener = ast.generics.clone();
    let where_clu = if let Some(w) = gener.where_clause.clone() {
        let where_ts = w.to_token_stream();
        quote! { #where_ts }
    } else { 
        quote! {}
    };

    let x = where_clu.to_string();

    if let syn::Data::Struct(s) = ast.data {
        if let syn::Fields::Named(fields) = s.fields {
            for f in fields.named {
               // let tokens = f.to_token_stream().to_string();
                if let Some(attr) = f.attrs.get(0) {
                    let attr_name = attr.path.to_token_stream().to_string();
                    if attr_name == "id" {
                        id = f.ident;
                        id_t_s = attr.tokens.to_string();
                        id_t_s.remove(0);
                        id_t_s.pop();
                    } else if attr_name == "data" {
                        data = f.ident;
                        data_t_s = attr.tokens.to_string();
                        data_t_s.remove(0);
                        data_t_s.pop();
                    }
                }
            }
        } else {
            panic!()
        }
    } else {
        panic!()
    }

    let id_ident = id.unwrap();
    let data_ident = data.unwrap();
    let id_t_ident = format_ident!("{id_t_s}");
    let data_t_ident =  format_ident!("{data_t_s}");
    //let where_ident = format_ident!("{where_clu}");
    
    let gen = quote! {
        impl #gener IObj for #name #gener 
        #where_clu
        {
            type IdType = #id_t_ident;
            type ValueType = #data_t_ident;
            fn get_id(self: &Self) -> Self::IdType {
                let _ = #x;
                self.#id_ident.clone()
            }
            fn get_obj_type(self: &Self) -> ObjType {ObjType::new::<Self>(ObjT::Normal)}
            fn get_copy_data_vec(self: &Self) -> Vec<Self::ValueType> {self.#data_ident.clone()}
            fn get_ref_data_vec(self: &Self) -> &Vec<Self::ValueType> {&self.#data_ident}
        }
    };
    gen.into()
}
