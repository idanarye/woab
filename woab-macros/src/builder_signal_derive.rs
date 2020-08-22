use syn::parse::Error;
use quote::quote;

pub fn impl_builder_signal_derive(ast: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let data_enum = if let syn::Data::Enum(data_enum) = &ast.data {
        data_enum
    } else {
        return Err(Error::new_spanned(ast, "BuilderSignal only supports enums"));
    };
    let enum_ident = &ast.ident;
    let match_arms = data_enum.variants.iter().map(|variant| {
        let fields = if let syn::Fields::Unnamed(fields) = &variant.fields {
            fields
        } else {
            return Err(Error::new_spanned(variant, "BuilderSignal only supports tuple variants (even if they are empty)"));
        };
        let variant_ident = &variant.ident;
        let ident_as_str = syn::LitStr::new(&variant_ident.to_string(), variant_ident.span());
        let field_from_arg_mappers = fields.unnamed.iter().enumerate().map(|(i, _field)| {
            Ok(quote! {
                args[#i].get().unwrap().unwrap()
            })
        }).collect::<Result<Vec<_>, Error>>()?;
        let num_fields = field_from_arg_mappers.len();
        Ok(quote! {
            #ident_as_str => Box::new(move |args| {
                if args.len() != #num_fields {
                    panic!("Expected {} to have {} parameters - got {}", #ident_as_str, #num_fields, args.len());
                }
                let msg = #enum_ident::#variant_ident(#(#field_from_arg_mappers),*);
                match tx.clone().try_send(msg) {
                    Ok(_) => None,
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => None,
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        panic!("Unable to send {} signal - channel is full", #ident_as_str);
                    },
                }
            }),
        })
    }).collect::<Result<Vec<_>, Error>>()?;
    Ok(quote! {
        impl #enum_ident {
            fn transmit_signal_in_stream_function(signal: &str, tx: tokio::sync::mpsc::Sender<Self>) -> Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>> {
                use tokio::sync::mpsc::error::TrySendError;
                match signal {
                    #(#match_arms)*
                    _ => Box::new(|_| {
                        None
                    }),
                }
            }
        }
    })
}
