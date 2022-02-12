use crate::util::{iter_attrs_parameters, path_to_single_string};
use quote::quote;
use syn::parse::Error;
use syn::spanned::Spanned;

pub fn impl_widgets_from_builder_derive(ast: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(fields),
        ..
    }) = &ast.data
    {
        fields
    } else {
        return Err(Error::new_spanned(
            ast,
            "WidgetsFromBuilder only supports structs with named fields",
        ));
    };
    let struct_ident = &ast.ident;
    let ctor_arms = fields
        .named
        .iter()
        .map(|field| {
            /* Handle renaming */
            let mut nested = false;
            let mut name = None;
            iter_attrs_parameters(&field.attrs, "widget", |attr_name, value| {
                match path_to_single_string(&attr_name)?.as_str() {
                    "nested" => {
                        if nested {
                            return Err(Error::new_spanned(value, "attribute `nested` can only be specified once"));
                        }
                        if value.is_some() {
                            return Err(Error::new_spanned(value, "attribute `nested` cannot have a value"));
                        }
                        nested = true;
                    }
                    "name" => {
                        let value = value.ok_or_else(|| Error::new_spanned(attr_name, "attribute `name` must have a value"))?;
                        if name.is_some() {
                            return Err(Error::new_spanned(value, "attribute `name` can only be specified once"));
                        }
                        name = Some(value);
                    }
                    _ => {
                        return Err(Error::new_spanned(attr_name, "unknown attribute"));
                    }
                }
                Ok(())
            })?;
            if nested && name.is_some() {
                return Err(Error::new_spanned(field, "`nested` and `name` are mutually exclusive"));
            }

            let field_ident = field
                .ident
                .as_ref()
                .ok_or_else(|| Error::new(field.span(), "Nameless field"))?;

            if nested {
                // NOTE: Not using `?` because it `into`es the error and the type checker does not like that.
                return Ok(quote! {
                    #field_ident: {
                        match std::convert::TryInto::try_into(builder) {
                            Ok(ok) => ok,
                            Err(err) => return Err(err),
                        }
                    },
                });
            }

            let field_type = &field.ty;
            let ident_as_str = match name {
                Some(syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(name),
                    ..
                })) => name,
                None => syn::LitStr::new(&field_ident.to_string(), field_ident.span()),
                _ => return Err(Error::new_spanned(name, "`name` attribute must have a string literal value")),
            };
            Ok(quote! {
                #field_ident: builder.object(#ident_as_str).ok_or_else(|| {
                    if let Some(object) = builder.object::<glib::Object>(#ident_as_str) {
                        use glib::object::ObjectExt;
                        woab::Error::IncorrectWidgetTypeInBuilder {
                            widget_id: #ident_as_str.to_owned(),
                            expected_type: <#field_type as glib::types::StaticType>::static_type(),
                            actual_type: object.type_(),
                        }
                    } else {
                        woab::Error::WidgetMissingInBuilder(#ident_as_str.to_owned())
                    }
                })?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(quote! {
        impl std::convert::TryFrom<&gtk4::Builder> for #struct_ident {
            type Error = woab::Error;

            fn try_from(builder: &gtk4::Builder) -> Result<Self, Self::Error> {
                Ok(Self {
                    #(#ctor_arms)*
                })
            }
        }

        impl std::convert::TryFrom<gtk4::Builder> for #struct_ident {
            type Error = woab::Error;

            fn try_from(builder: gtk4::Builder) -> Result<Self, Self::Error> {
                <Self as std::convert::TryFrom<&gtk4::Builder>>::try_from(&builder)
            }
        }
    })
}
