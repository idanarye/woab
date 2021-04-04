use quote::quote;
use syn::parse::Error;

pub fn impl_removable_derive(ast: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let type_ident = &ast.ident;

    let mut removable_attr = None;

    for attr in ast.attrs.iter() {
        if let Some(path_ident) = attr.path.get_ident() {
            if path_ident == "removable" {
                if removable_attr.is_some() {
                    return Err(Error::new_spanned(attr, "There can only be one #[removable(...)] attribute"));
                }
                removable_attr = Some(attr);
                continue;
            }
        }
    }

    let removable_attr =
        removable_attr.ok_or_else(|| Error::new_spanned(ast, "#[removable(...)] is mandatory when deriving Removable"))?;
    let widget_to_remove = &removable_attr.tokens;

    Ok(quote! {
        impl actix::Handler<woab::Remove> for #type_ident {
            type Result = ();

            fn handle(&mut self, _: woab::Remove, ctx: &mut Self::Context) -> Self::Result {
                use gtk::prelude::*;
                use actix::prelude::*;

                let widget = &#widget_to_remove;
                if let Some(parent) = widget.get_parent() {
                    let parent = parent.downcast::<gtk::Container>().unwrap();
                    let widget = widget.clone();
                    ctx.stop();
                    woab::schedule_outside(move || {
                        parent.remove(&widget);
                    });
                }
            }
        }
    })
}
