mod widgets_from_builder_derive;
mod builder_signal_derive;
mod factories_derive;
mod removable_derive;
mod util;

#[proc_macro_derive(WidgetsFromBuilder, attributes(widget))]
pub fn derive_widgets_from_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match widgets_from_builder_derive::impl_widgets_from_builder_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(BuilderSignal, attributes(signal))]
pub fn derive_builder_signal(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match builder_signal_derive::impl_builder_signal_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(Factories, attributes(factory))]
pub fn derive_factories(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match factories_derive::impl_factories_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(Removable, attributes(removable))]
pub fn derive_removable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match removable_derive::impl_removable_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
