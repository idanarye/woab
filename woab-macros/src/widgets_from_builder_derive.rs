use syn::parse::Error;
use syn::spanned::Spanned;
use quote::quote;

pub fn impl_widgets_from_builder_derive(ast: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(fields),
        ..
    }) = &ast.data {
        fields
    } else {
        return Err(Error::new_spanned(ast, "WidgetsFromBuilder only supports structs with named fields"));
    };
    let struct_ident = &ast.ident;
    let ctor_arms = fields.named.iter().map(|field| {
        let field_ident = field.ident.as_ref().ok_or_else(|| Error::new(field.span(), "Nameless field"))?;
        let ident_as_str = syn::LitStr::new(&field_ident.to_string(), field_ident.span());
        Ok(quote!{
            #field_ident: builder.get_object(#ident_as_str).ok_or(woab::errors::WidgetMissingInBuilder(#ident_as_str))?,
        })
    }).collect::<Result<Vec<_>, Error>>()?;
    Ok(quote!{
        impl std::convert::TryFrom<&gtk::Builder> for #struct_ident {
            type Error = woab::errors::WidgetMissingInBuilder;

            fn try_from(builder: &gtk::Builder) -> Result<Self, Self::Error> {
                use gtk::prelude::BuilderExtManual;
                Ok(Self {
                    #(#ctor_arms)*
                })
            }
        }
    })
}
