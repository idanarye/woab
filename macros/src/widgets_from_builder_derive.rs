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
        let field_type = &field.ty;
        let ident_as_str = syn::LitStr::new(&field_ident.to_string(), field_ident.span());
        Ok(quote!{
            #field_ident: builder.get_object(#ident_as_str).ok_or_else(|| {
                if let Some(object) = builder.get_object::<glib::Object>(#ident_as_str) {
                    use glib::object::ObjectExt;
                    woab::Error::IncorrectWidgetTypeInBuilder {
                        widget_id: #ident_as_str,
                        expected_type: <#field_type as glib::types::StaticType>::static_type(),
                        actual_type: object.get_type(),
                    }
                } else {
                    woab::Error::WidgetMissingInBuilder(#ident_as_str)
                }
            })?,
        })
    }).collect::<Result<Vec<_>, Error>>()?;
    Ok(quote!{
        impl std::convert::TryFrom<&gtk::Builder> for #struct_ident {
            type Error = woab::Error;

            fn try_from(builder: &gtk::Builder) -> Result<Self, Self::Error> {
                use gtk::prelude::BuilderExtManual;
                Ok(Self {
                    #(#ctor_arms)*
                })
            }
        }
    })
}
