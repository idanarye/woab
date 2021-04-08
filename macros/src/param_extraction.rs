use quote::quote;
use syn::parse::Error;

pub struct Input {
    params: syn::punctuated::Punctuated<SingleParam, syn::token::Comma>,
}

impl syn::parse::Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Input {
            params: syn::punctuated::Punctuated::parse_terminated(input)?,
        })
    }
}

#[derive(Debug)]
enum SingleParam {
    Extract {
        pat: syn::Pat,
        ty: syn::Type,
    },
    Ignore {
        pat: syn::Pat,
    },
}

impl syn::parse::Parse for SingleParam {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let pat: syn::Pat = input.parse()?;
        let lookahead = input.lookahead1();
        Ok(if lookahead.peek(syn::token::Colon) {
            let _: syn::token::Colon = input.parse()?;
            let ty = input.parse()?;
            SingleParam::Extract { pat, ty }
        } else {
            SingleParam::Ignore { pat }
        })
    }
}

impl Input {
    pub fn impl_param_extraction(&self) -> Result<proc_macro2::TokenStream, Error> {
        let mut result = quote!(());
        for param in self.params.iter().rev() {
            result = match param {
                SingleParam::Extract { pat, ty } => {
                    quote! {
                        (#pat, core::marker::PhantomData::<#ty>, #result)
                    }
                }
                SingleParam::Ignore { pat } => {
                    quote! {
                        ((#pat,), #result)
                    }
                }
            };
        }
        Ok(result)
    }
}
