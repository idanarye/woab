use crate::util::{iter_attrs_parts, path_to_single_string};
use quote::{quote, quote_spanned};
use syn::parse::Error;
use syn::spanned::Spanned;

pub fn impl_prop_sync_derive(ast: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(fields),
        ..
    }) = &ast.data
    {
        fields
    } else {
        return Err(Error::new_spanned(ast, "PropSync only supports structs with named fields"));
    };
    let mut fields_to_sync = Vec::new();
    for field in fields.named.iter() {
        let mut getter = false;
        let mut setter = false;
        let mut field_property = None;
        iter_attrs_parts(&field.attrs, "prop_sync", |expr| {
            match expr {
                syn::Expr::Path(path) => match path_to_single_string(&path.path)?.as_str() {
                    "get" => {
                        getter = true;
                    }
                    "set" => {
                        setter = true;
                    }
                    _ => {
                        return Err(Error::new_spanned(path, "unknown attribute"));
                    }
                },
                syn::Expr::Cast(syn::ExprCast { expr, ty, .. }) => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        attrs: _,
                        lit: syn::Lit::Str(property),
                    }) = *expr
                    {
                        field_property = Some((property, *ty));
                    } else {
                        return Err(Error::new_spanned(
                                expr,
                                "expected a string literal (representing a GTK property)",
                        ));
                    }
                }
                _ => {
                    return Err(Error::new_spanned(expr, "illegal attribute option"));
                }
            }
            Ok(())
        })?;
        if getter || setter {
            fields_to_sync.push(FieldToSync {
                ident: field.ident.as_ref().unwrap(),
                ty: &field.ty,
                property: field_property,
                getter,
                setter,
            });
        }
    }
    let setter = gen_setter(ast, &fields_to_sync)?;
    let getter = gen_getter(ast, &fields_to_sync)?;
    Ok(quote! {
        #setter
        #getter
    })
}

#[derive(Debug)]
struct FieldToSync<'a> {
    ident: &'a syn::Ident,
    ty: &'a syn::Type,
    property: Option<(syn::LitStr, syn::Type)>,
    getter: bool,
    setter: bool,
}

fn gen_setter(ast: &syn::DeriveInput, fields: &[FieldToSync]) -> Result<proc_macro2::TokenStream, Error> {
    if !fields.iter().any(|f| f.setter) {
        return Ok(quote!());
    }

    let struct_name = &ast.ident;
    let setter_name = format!("{}PropSetter", struct_name);
    let setter_name = syn::Ident::new(&setter_name, ast.ident.span());
    let vis = &ast.vis;

    let mut lifetime = None;

    let mut struct_fields = Vec::new();
    let mut prop_assignment = Vec::new();

    for field in fields.iter() {
        if !field.setter {
            continue;
        }
        let ident = field.ident;
        let field_type = field.ty;
        if let Some((prop, ty)) = &field.property {
            if let syn::Type::Reference(ty_ref) = ty {
                let mut ty_ref = ty_ref.clone();
                ty_ref.lifetime = Some(
                    lifetime
                        .get_or_insert_with(|| syn::Lifetime::new("'a", proc_macro2::Span::call_site()))
                        .clone(),
                );
                struct_fields.push(quote! {
                    #ident: #ty_ref
                });
            } else {
                struct_fields.push(quote! {
                    #ident: #ty
                });
            }
            prop_assignment.push(quote! {
                glib::object::ObjectExt::set_property(&self.#ident, #prop, &setter.#ident);
            });
        } else {
            let lifetime = lifetime.get_or_insert_with(|| syn::Lifetime::new("'a", proc_macro2::Span::call_site()));
            let as_trait = quote_spanned! { field_type.span() =>
                <#field_type as woab::prop_sync::SetProps<#lifetime>>
            };
            struct_fields.push(quote_spanned! { field_type.span() =>
                #ident: #as_trait::SetterType
            });
            prop_assignment.push(quote_spanned! { field_type.span() =>
                #as_trait::set_props(&self.#ident, &setter.#ident);
            });
        }
    }

    let lifetime_for_trait = if let Some(lifetime) = &lifetime {
        lifetime.clone()
    } else {
        syn::Lifetime::new("'static", proc_macro2::Span::call_site())
    };

    Ok(quote! {
        #vis struct #setter_name <#lifetime> {
            #(#struct_fields),*
        }

        impl<'a> woab::prop_sync::SetProps<'a> for #struct_name {
            type SetterType = #setter_name<#lifetime>;

            fn set_props(&self, setter: &Self::SetterType) {
                #(#prop_assignment)*
            }
        }

        impl #struct_name {
            #vis fn set_props<#lifetime>(&self, setter: &#lifetime <Self as woab::prop_sync::SetProps<#lifetime_for_trait>>::SetterType) {
                <Self as woab::prop_sync::SetProps>::set_props(self, setter);
            }
        }
    })
}

fn gen_getter(ast: &syn::DeriveInput, fields: &[FieldToSync]) -> Result<proc_macro2::TokenStream, Error> {
    if !fields.iter().any(|f| f.getter) {
        return Ok(quote!());
    }

    let struct_name = &ast.ident;
    let getter_name = format!("{}PropGetter", struct_name);
    let getter_name = syn::Ident::new(&getter_name, ast.ident.span());
    let vis = &ast.vis;

    let mut struct_fields = Vec::new();
    let mut field_from_prop = Vec::new();

    for field in fields.iter() {
        if !field.getter {
            continue;
        }
        let ident = field.ident;
        let field_type = field.ty;
        if let Some((prop, ty)) = &field.property {
            if let syn::Type::Reference(ty_ref) = ty {
                let ty = &ty_ref.elem;
                struct_fields.push(quote! {
                    #ident: <#ty as std::borrow::ToOwned>::Owned
                });
            } else {
                struct_fields.push(quote! {
                    #ident: #ty
                });
            }
            field_from_prop.push(quote! {
                #ident: glib::object::ObjectExt::property::<#ty>(&self.#ident, #prop)
            });
        } else {
            let as_trait = quote_spanned! { field_type.span() =>
                <#field_type as woab::prop_sync::GetProps>
            };
            struct_fields.push(quote_spanned! { field_type.span() =>
                #ident: #as_trait::GetterType
            });
            field_from_prop.push(quote_spanned! { field_type.span() =>
                #ident: #as_trait::get_props(&self.#ident)
            });
        }
    }

    Ok(quote! {
        #vis struct #getter_name {
            #(#struct_fields),*
        }

        impl woab::prop_sync::GetProps for #struct_name {
            type GetterType = #getter_name;

            fn get_props(&self) ->  Self::GetterType {
                #getter_name {
                    #(#field_from_prop),*
                }
            }
        }

        impl #struct_name {
            #vis fn get_props(&self) -> <Self as woab::prop_sync::GetProps>::GetterType {
                <Self as woab::prop_sync::GetProps>::get_props(self)
            }
        }
    })
}
