/// Stub proc-macro derive for `ScoreDebug`.
///
/// This generates a trivial no-op implementation of `score_log::fmt::ScoreDebug`
/// so the FEO source code (which derives `ScoreDebug` everywhere) compiles under
/// plain cargo when `score_log` is provided by the local stub crate instead of
/// the Bazel-only `@score_baselibs_rust//src/log/score_log`.
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(ScoreDebug)]
pub fn derive_score_debug(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics score_log::fmt::ScoreDebug for #name #ty_generics #where_clause {
            fn fmt(
                &self,
                _w: &mut dyn score_log::fmt::ScoreWrite,
                _spec: &score_log::fmt::FormatSpec,
            ) -> score_log::fmt::Result {
                Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}
