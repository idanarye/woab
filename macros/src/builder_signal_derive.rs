use syn::parse::Error;
use syn::spanned::Spanned;
use quote::quote;

use crate::util::{
    path_to_single_string,
    iter_attrs_parameters
};

pub fn impl_builder_signal_derive(ast: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let data_enum = if let syn::Data::Enum(data_enum) = &ast.data {
        data_enum
    } else {
        return Err(Error::new_spanned(ast, "BuilderSignal only supports enums"));
    };
    let enum_ident = &ast.ident;
    let match_arms = data_enum.variants.iter().map(|variant| {
        let mut inhibit = None;
        iter_attrs_parameters(&variant.attrs, "signal", |name, value| {
            match path_to_single_string(&name)?.as_str() {
                "inhibit" => {
                    let value = value.ok_or_else(|| Error::new_spanned(name, "`inhibit` must have a value"))?;
                    if inhibit.is_some() {
                        return Err(Error::new_spanned(value, "`inhibit` already set"));
                    }
                    inhibit = Some(value);
                }
                _ => {
                    return Err(Error::new_spanned(name, "unknown argument"));
                }
            }
            Ok(())
        })?;
        let signal_return_value = if let Some(inhibit) = inhibit {
            quote! {
                Some(glib::value::ToValue::to_value(&#inhibit))
            }
        } else {
            quote! {
                if let Some(inhibit_dlg) = &inhibit_dlg {
                    if let Some(gtk::Inhibit(inhibit)) = inhibit_dlg(&signal) {
                        Some(glib::value::ToValue::to_value(&inhibit))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        let variant_ident = &variant.ident;
        let ident_as_str = syn::LitStr::new(&variant_ident.to_string(), variant_ident.span());
        let msg_construction = match &variant.fields {
            syn::Fields::Unnamed(fields) => {
                let field_from_arg_mappers = fields.unnamed.iter().enumerate().map(|(i, field)| {
                    let mut is_event = false;
                    iter_attrs_parameters(&field.attrs, "signal", |name, value| {
                        match path_to_single_string(&name)?.as_str() {
                            "event" => {
                                if value.is_some() {
                                    return Err(Error::new_spanned(value, "`event` cannot have a value"))?;
                                }
                                if is_event {
                                    return Err(Error::new_spanned(value, "`event` already set"));
                                }
                                is_event = true;
                            }
                            _ => {
                                return Err(Error::new_spanned(name, "unknown argument"));
                            }
                        }
                        Ok(())
                    })?;

                    let type_error = syn::LitStr::new(&format!("Wrong type for paramter {} of {}", i, variant_ident), field.ty.span());
                    let none_error = syn::LitStr::new(&format!("Paramter {} of {} is None", i, variant_ident), field.ty.span());

                    let mut result = quote! {
                        args[#i].get().expect(#type_error).expect(#none_error)
                    };
                    if is_event {
                        result = quote! {
                            {
                                let event: gdk::Event = #result;
                                event.downcast().expect(#type_error)
                            }
                        }
                    }
                    Ok(result)
                }).collect::<Result<Vec<_>, Error>>()?;
                let num_fields = field_from_arg_mappers.len();
                quote! {
                    {
                        if args.len() != #num_fields {
                            panic!("Expected {} to have {} parameters - got {}", #ident_as_str, #num_fields, args.len());
                        }
                        #enum_ident::#variant_ident(#(#field_from_arg_mappers),*)
                    }
                }
            }
            syn::Fields::Unit => quote!(#enum_ident::#variant_ident),
            syn::Fields::Named(_) => return Err(Error::new_spanned(variant, "BuilderSignal only supports unit or tuple variants (even if they are empty)")),
        };
        Ok(quote! {
            #ident_as_str => Some(Box::new(move |args| {
                let signal = #msg_construction;
                let return_value = #signal_return_value;
                match tx.clone().try_send(signal) {
                    Ok(_) => return_value,
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        panic!("Unable to send {} signal - channel is closed", #ident_as_str);
                    },
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        panic!("Unable to send {} signal - channel is full", #ident_as_str);
                    },
                }
            })),
        })
    }).collect::<Result<Vec<_>, Error>>()?;
    Ok(quote! {
        impl woab::BuilderSignal for #enum_ident {
            fn transmit_signal_in_stream_function(signal: &str, tx: tokio::sync::mpsc::Sender<Self>, inhibit_dlg: Option<std::rc::Rc<dyn Fn(&Self) -> Option<gtk::Inhibit>>>) -> Option<Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>> {
                use tokio::sync::mpsc::error::TrySendError;
                match signal {
                    #(#match_arms)*
                    _ => None,
                }
            }
        }
    })
}
