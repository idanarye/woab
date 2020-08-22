mod widgets_from_builder_derive;
mod builder_signal_derive;

#[proc_macro_derive(WidgetsFromBuilder)]
pub fn derive_widgets_from_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match widgets_from_builder_derive::impl_widgets_from_builder_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(BuilderSignal)]
pub fn derive_builder_signal(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match builder_signal_derive::impl_builder_signal_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
