use proc_macro::TokenStream;

#[proc_macro]
pub fn sql(_input: TokenStream) -> TokenStream {
    return "todo!(\"Renoir SQL macro not yet implemented\");".parse().unwrap();
}