use quote::quote;
use syn::parse::Error;
use syn::spanned::Spanned;

pub fn impl_factories_derive(ast: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(fields),
        ..
    }) = &ast.data
    {
        fields
    } else {
        return Err(Error::new_spanned(ast, "Factories only supports structs with named fields"));
    };
    let struct_ident = &ast.ident;
    let num_factories = fields.named.len();

    let single_buffer = quote! {Vec::new()};
    let buffers = std::iter::repeat(&single_buffer).take(num_factories);

    let mut match_arms = Vec::with_capacity(num_factories);
    let mut deconstruct_buffers_array = Vec::new();
    let mut ctor_arms = Vec::new();

    for (i, field) in fields.named.iter().enumerate() {
        let field_ident = field
            .ident
            .as_ref()
            .ok_or_else(|| Error::new(field.span(), "Nameless field"))?;
        let mut strings_that_match = vec![syn::LitStr::new(&field_ident.to_string(), field_ident.span())];

        for attr in field.attrs.iter() {
            if !attr.path().get_ident().map_or(false, |ident| ident == "factory") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                let meta_name = meta.path.get_ident().map(|ident| ident.to_string());
                match meta_name.as_deref() {
                    Some("extra") => {
                        meta.parse_nested_meta(|extra_item| {
                            if let Some(ident) = extra_item.path.get_ident() {
                                strings_that_match.push(syn::LitStr::new(&ident.to_string(), ident.span()));
                            }
                            Ok(())
                        })?;
                    }
                    _ => return Err(Error::new_spanned(meta.path, "Unsupported parameter")),
                }
                Ok(())
            })?;
        }
        match_arms.push(quote! {
            #(#strings_that_match)|* => Some(#i),
        });
        deconstruct_buffers_array.push(field_ident);
        ctor_arms.push(quote! {
            #field_ident: String::from_utf8(#field_ident)?.into(),
        });
    }

    Ok(quote! {
        impl #struct_ident {
            pub fn read(buf_read: impl std::io::BufRead) -> Result<Self, woab::Error> {
                let mut buffers = [#(#buffers),*];
                woab::dissect_builder_xml(buf_read, &mut buffers, |id| match id {
                    #(#match_arms)*
                    _ => None,
                })?;
                let [#(#deconstruct_buffers_array),*] = buffers;
                Ok(Self {
                    #(#ctor_arms)*
                })
            }
        }
    })
}
