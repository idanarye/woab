use syn::parse::Error;
use syn::spanned::Spanned;
use quote::quote;

pub fn impl_factories_derive(ast: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(fields),
        ..
    }) = &ast.data {
        fields
    } else {
        return Err(Error::new_spanned(ast, "Factories only supports structs with named fields"));
    };
    let struct_ident = &ast.ident;
    let num_factories = fields.named.len();

    let single_buffer = quote!{Vec::new()};
    let buffers = std::iter::repeat(&single_buffer).take(num_factories);

    let mut match_arms = Vec::with_capacity(num_factories);
    let mut deconstruct_buffers_array = Vec::new();
    let mut ctor_arms = Vec::new();

    for (i, field) in fields.named.iter().enumerate() {
        let field_ident = field.ident.as_ref().ok_or_else(|| Error::new(field.span(), "Nameless field"))?;
        let mut strings_that_match = vec![
            syn::LitStr::new(&field_ident.to_string(), field_ident.span())
        ];

        for attr in field.attrs.iter() {
            if !attr.path.get_ident().map_or(false, |ident| ident == "factory") {
                continue;
            }
            let meta = if let syn::Meta::List(meta) = attr.parse_meta()? {
                meta
            } else {
                return Err(Error::new_spanned(attr, "Only list style (`#[factory(...)]`) is supported"));
            };

            for list_item in meta.nested.iter() {
                let meta = if let syn::NestedMeta::Meta(meta) = list_item {
                    meta
                } else {
                    return Err(Error::new_spanned(list_item, "Literals are not supported directly inside factory attribute"));
                };
                let meta_name = meta.path().get_ident().map(|ident| ident.to_string());
                match meta_name.as_deref() {
                    Some("extra") => {
                        if let syn::Meta::List(extra) = meta {
                            for extra_item in extra.nested.iter() {
                                if let syn::NestedMeta::Meta(syn::Meta::Path(extra_path)) = extra_item {
                                    if let Some(ident) = extra_path.get_ident() {
                                        strings_that_match.push(syn::LitStr::new(&ident.to_string(), ident.span()));
                                        continue;
                                    }
                                }
                                return Err(Error::new_spanned(extra_item, "extra items must be identifiers"));
                            }
                        } else {
                            return Err(Error::new_spanned(meta, "extra must be list (`#[factory(extra(...))]`)"));
                        }
                    },
                    _ => return Err(Error::new_spanned(meta.path(), "Unsupported parameter")),
                }
            }

        }
        match_arms.push(quote!{
            #(#strings_that_match)|* => Some(#i),
        });
        deconstruct_buffers_array.push(field_ident);
        ctor_arms.push(quote!{
            #field_ident: String::from_utf8(#field_ident).unwrap().into(),
        });
    }

    Ok(quote!{
        impl #struct_ident {
            pub fn read(buf_read: impl std::io::BufRead) -> Self {
                let mut buffers = [#(#buffers),*];
                woab::dissect_builder_xml(buf_read, &mut buffers, |id| match id {
                    #(#match_arms)*
                    _ => None,
                });
                let [#(#deconstruct_buffers_array),*] = buffers;
                Self {
                    #(#ctor_arms)*
                }
            }
        }
    })
}
