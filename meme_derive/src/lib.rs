extern crate proc_macro;

use std::borrow::Borrow;

use quote::{quote, ToTokens};

use syn::{parse::Parse, AngleBracketedGenericArguments, DeriveInput, Ident};

struct GeneType {
    pub _ident: Ident,
    pub gene: AngleBracketedGenericArguments
}
impl Parse for GeneType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(GeneType {
            _ident: input.parse()?,
            gene: input.parse()?
        })
    }
}

#[proc_macro_derive(IObj, attributes(tag, amount, obj_type))]
pub fn iobj_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let obj_type_attr = ast.attrs.iter().find(|a| {
       a.path().is_ident("obj_type")
    }).map_or(quote!{ meme::core::DEFAULT_GROUP }, |a| {
        let list = a.meta.require_list().expect("obj_type属性不完整").tokens.to_token_stream();
        quote! { #list }
    });

    let tag_field = if let syn::Data::Struct(s) = ast.data.borrow() {
        if let syn::Fields::Named(fields) = &s.fields {
            fields.named.iter().find(|f| { 
                f.attrs.iter().any(|a| a.path().is_ident("tag"))
            }).cloned()
        } else {
            None
        }
    } else {
        None
    }.expect("缺少tag属性");

    let tag = tag_field.ident.expect("tag属性不完整");
    let tag_type = tag_field.ty;
    
    let amount_field = if let syn::Data::Struct(s) = ast.data {
        if let syn::Fields::Named(fields) = s.fields {
            fields.named.iter().find(|f| { 
                f.attrs.iter().any(|a| a.path().is_ident("amount"))
            }).cloned()
        } else {
            None
        }
    } else {
        None
    };

    let (amount, amount_type) = if let Some(a) = amount_field {
        let member = a.ident.expect("amount属性不完整");
        let ty = a.ty;
        let ret = quote! { self.#member };
        let unit_t = quote! { #ty };
        (ret, unit_t)
    } else {
        let ret = quote! { 1u32 };
        let unit_t = quote! { u32 };
        (ret, unit_t)
    };

    (quote! {
        impl #impl_generics meme::core::IObj for #name #ty_generics #where_clause {
            type Tag = #tag_type;
            type Unit = #amount_type;
            fn obj_tag(&self) -> &Self::Tag { &self.#tag }
            fn obj_amount(&self) -> Self::Unit { #amount }
            fn obj_type(&self) -> meme::core::ObjType { meme::core::ObjType::new::<Self>(&#obj_type_attr) }
            fn as_any(&self) -> &dyn std::any::Any { self }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
        }
    }).into()
}

#[proc_macro_derive(IRule, attributes(condition, effect, obj_tag_type, obj_unit_type))]
pub fn irule_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let cond_field = if let syn::Data::Struct(s) = ast.data.borrow() {
        if let syn::Fields::Named(fields) = &s.fields {
            fields.named.iter().find(|f| { 
                f.attrs.iter().any(|a| a.path().is_ident("condition"))
            }).cloned()
        } else {
            None
        }
    } else {
        None
    }.expect("缺少condition属性");

    let cond = cond_field.ident.expect("condition属性不完整");
    let cond_type = cond_field.ty;

    let cond_ty_indent = syn::parse2::<GeneType>(cond_type.to_token_stream());

    let obj_tag_type = ast.attrs.iter().find(|a| {
        a.path().is_ident("obj_tag_type")
    }).map(|a| {
        let list = a.meta.require_list().expect("obj_tag_type属性不完整").tokens.to_token_stream();
        quote! { #list }
    }).unwrap_or({
        cond_ty_indent.as_ref().map(|c| {
            if let Some(t) = c.gene.args.get(0) {
                quote! { #t }
            } else {
                quote! { u32 }
            }
        } ).unwrap_or(quote! { u32 })
    });

    let obj_unit_type = ast.attrs.iter().find(|a| {
        a.path().is_ident("obj_unit_type")
    }).map(|a| {
        let list = a.meta.require_list().expect("obj_unit_type属性不完整").tokens.to_token_stream();
        quote! { #list }
    }).unwrap_or({
        cond_ty_indent.map(|c| {
            if let Some(t) = c.gene.args.get(1) {
                quote! { #t }
            } else {
                quote! { u32 }
            }
        } ).unwrap_or(quote! { u32 })
    });

    let eff_field = if let syn::Data::Struct(s) = ast.data.borrow() {
        if let syn::Fields::Named(fields) = &s.fields {
            fields.named.iter().find(|f| { 
                f.attrs.iter().any(|a| a.path().is_ident("effect"))
            }).cloned()
        } else {
            None
        }
    } else {
        None
    }.expect("缺少effect属性");

    let eff = eff_field.ident.expect("effect属性不完整");
    let eff_type = eff_field.ty;

    (quote! {
        impl #impl_generics meme::core::IRule for #name #ty_generics #where_clause {
            type ObjTag = #obj_tag_type;
            type ObjUnit = #obj_unit_type;
            type Condition = #cond_type;
            type Effect = #eff_type;
            fn condition(&self) -> &Self::Condition { &self.#cond }
            fn effect(&self) -> &Self::Effect { &self.#eff }
        }
    }).into()
}


#[proc_macro_derive(IntoSRStr)]
pub fn into_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let arms = parse_arms(&ast).expect("无法解析");

    (quote! {
        impl #impl_generics Into<&'static str> for #name #ty_generics #where_clause {
            fn into(self: Self) -> &'static str {
                match self {
                    #(#arms)*
                }
            }
        }
    }).into()
}

fn parse_arms(ast: &DeriveInput) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let name = &ast.ident;
    let mut arms = Vec::new();

    let variants = if let syn::Data::Enum(e) = &ast.data {
        Some(&e.variants)
    } else {
        None
    }.expect("不是枚举");

    for variant in variants {
        let ident = &variant.ident;
       
        let output = format!("{}::{}", name, ident);
     
        let params = match variant.fields {
            syn::Fields::Unit => quote! {},
            syn::Fields::Unnamed(..) => quote! { (..) },
            syn::Fields::Named(..) => quote! { {..} },
        };

        arms.push(quote! { #name::#ident #params => #output, });
    }

    Ok(arms)
}
