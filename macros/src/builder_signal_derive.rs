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
    let vec_of_tuples = data_enum.variants.iter().map(|variant| {
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
                if let Some(gtk::Inhibit(inhibit)) = inhibit_dlg(&signal) {
                    Some(glib::value::ToValue::to_value(&inhibit))
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
                    enum ConversionType {
                        NoConversion,
                        Event,
                        Variant,
                    }

                    impl ConversionType {
                        fn name(&self) -> &'static str {
                            match self {
                                ConversionType::NoConversion => "",
                                ConversionType::Event => "event",
                                ConversionType::Variant => "variant",
                            }
                        }
                    }

                    let mut conversion = ConversionType::NoConversion;
                    iter_attrs_parameters(&field.attrs, "signal", |name, value| {
                        let setting = path_to_single_string(&name)?;
                        let setting = setting.as_str();
                        match setting {
                            "event" | "variant" => {
                                if value.is_some() {
                                    return Err(Error::new_spanned(value, format!("{:?} cannot have a value", setting)))?;
                                }
                                if let ConversionType::NoConversion = conversion {
                                    conversion = match setting {
                                        "event" => ConversionType::Event,
                                        "variant" => ConversionType::Variant,
                                        _ => panic!("already ruled that out in the outer `match`"),
                                    };
                                } else {
                                    return Err(Error::new_spanned(value, format!("already converting to {}", conversion.name())));
                                }
                            }
                            _ => {
                                return Err(Error::new_spanned(name, "unknown argument"));
                            }
                        }
                        Ok(())
                    })?;

                    let type_error = syn::LitStr::new(&format!("Wrong type for paramter {} of {}", i, variant_ident), field.ty.span());
                    let none_error = syn::LitStr::new(&format!("Paramter {} of {} is None", i, variant_ident), field.ty.span());

                    let result = quote! {
                        args[#i].get().expect(#type_error).expect(#none_error)
                    };
                    let result = match conversion {
                        ConversionType::NoConversion => result,
                        ConversionType::Event => quote! {
                            {
                                let event: gdk::Event = #result;
                                event.downcast().expect(#type_error)
                            }
                        },
                        ConversionType::Variant => quote! {
                            {
                                let variant: glib::Variant = #result;
                                variant.get().expect(#type_error)
                            }
                        },
                    };
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
        Ok((
            /* Match arms */
            quote! {
                #ident_as_str => Ok(Box::new(move |args| {
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
            },
            /* Signal names */
            quote! {
                #ident_as_str,
            },
        ))
    }).collect::<Result<Vec<_>, Error>>()?;
    /* We cannot use unzip with error handling, so here's a workaround */
    let (match_arms, signal_names) = vec_of_tuples.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
    Ok(quote! {
        impl woab::BuilderSignal for #enum_ident {
            fn bridge_signal(signal: &str, tx: tokio::sync::mpsc::Sender<Self>, inhibit_dlg: impl 'static + Fn(&Self) -> Option<gtk::Inhibit>) -> Result<woab::RawSignalCallback, woab::Error> {
                use tokio::sync::mpsc::error::TrySendError;
                match signal {
                    #(#match_arms)*
                    _ => Err(woab::Error::NoSuchSignalError(core::any::type_name::<Self>(), signal.to_owned())),
                }
            }

            fn list_signals() -> &'static [&'static str] {
                &[
                    #(#signal_names)*
                ]
            }
        }

        impl #enum_ident {
            fn connector() -> woab::BuilderSingalConnector<Self, (), ()> {
                <Self as woab::BuilderSignal>::connector()
            }
        }
    })
}
