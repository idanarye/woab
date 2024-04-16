use syn::parse::Error;

pub fn path_to_single_string(path: &syn::Path) -> Result<String, Error> {
    if path.leading_colon.is_some() {
        return Err(Error::new_spanned(path, "cannot have leading colon"));
    }
    let mut it = path.segments.iter();
    let segment = it.next().ok_or_else(|| Error::new_spanned(path, "cannot be empty"))?;
    if it.next().is_some() {
        // Multipart path
        return Err(Error::new_spanned(path, "cannot have multiple parts"));
    }
    if segment.arguments != syn::PathArguments::None {
        return Err(Error::new_spanned(path, "cannot have arguments"));
    }
    Ok(segment.ident.to_string())
}

pub fn iter_attrs_parts(
    attrs: &[syn::Attribute],
    look_for: &str,
    mut dlg: impl FnMut(syn::Expr) -> Result<(), Error>,
) -> Result<(), Error> {
    for attr in attrs.iter() {
        if !attr.path().get_ident().map_or(false, |ident| ident == look_for) {
            continue;
        }
        for expr in attr.parse_args_with(|p: syn::parse::ParseStream| {
            syn::punctuated::Punctuated::<syn::Expr, syn::token::Comma>::parse_terminated(p)
        })? {
            dlg(expr)?;
        }
    }
    Ok(())
}

pub fn iter_attrs_parameters(
    attrs: &[syn::Attribute],
    look_for: &str,
    mut dlg: impl FnMut(syn::Path, Option<syn::Expr>) -> Result<(), Error>,
) -> Result<(), Error> {
    iter_attrs_parts(attrs, look_for, |expr| {
        match expr {
            syn::Expr::Assign(assign) => {
                let path = if let syn::Expr::Path(path) = *assign.left {
                    path
                } else {
                    return Err(Error::new_spanned(assign.left, "Not a valid name"));
                };
                dlg(path.path, Some(*assign.right))?;
            }
            syn::Expr::Path(path) => {
                dlg(path.path, None)?;
            }
            _ => {
                return Err(Error::new_spanned(expr, "Expected (<...>=<...>)"));
            }
        }
        Ok(())
    })
}
