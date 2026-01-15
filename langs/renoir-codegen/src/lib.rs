use proc_macro2::TokenStream;
use renoir_ir::Program;
use syn::Ident;

/// Generate Renoir code from a Program IR using a provided context
pub fn generate_program_with_context(program: &Program, ctx_name: &Ident) -> TokenStream {
    todo!()
}