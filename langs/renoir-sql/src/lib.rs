mod parser;

use parser::parse_sql;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::{Parse, ParseStream}, parse_macro_input, Ident, LitStr, Token};

/// Input for the sql! macro: context_name, "SQL query"
struct SqlMacroInput {
    ctx_name: Ident,
    _comma: Token![,],
    sql_query: LitStr,
}

impl Parse for SqlMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(SqlMacroInput {
            ctx_name: input.parse()?,
            _comma: input.parse()?,
            sql_query: input.parse()?,
        })
    }
}

/// SQL macro to parse SQL and generate Renoir streaming code
/// 
/// The user must create the StreamContext and pass it to the macro.
/// 
/// # Example
/// ```rust,ignore
/// use renoir::prelude::*;
/// use renoir_sql::sql;
/// 
/// let config = RuntimeConfig::local(4).unwrap();
/// let ctx = StreamContext::new(config);
/// 
/// sql! { ctx,
///     "CREATE SOURCE users (id i64, name String) WITH (connector = 'csv', path = 'users.csv');
///      CREATE SINK output (id i64, name String) WITH (connector = 'csv', path = 'output.csv');
///      INSERT INTO output (id, name) SELECT id, name FROM users WHERE id > 100;"
/// }
/// 
/// ctx.execute_blocking();
/// ```
#[proc_macro]
pub fn sql(input: TokenStream) -> TokenStream {
    // Parse the input as (context_name, "SQL string")
    let SqlMacroInput { ctx_name, sql_query, .. } = parse_macro_input!(input as SqlMacroInput);
    let sql_content = sql_query.value();
    
    // Parse SQL into IR
    let program = match parse_sql(&sql_content) {
        Ok(prog) => prog,
        Err(e) => {
            let error_msg = format!("SQL parse error: {:?}", e);
            return quote! {
                compile_error!(#error_msg);
            }.into();
        }
    };
    
    // Generate Renoir code from IR using the provided context
    let renoir_code = renoir_codegen::generate_program_with_context(&program, &ctx_name);
    
    // Return the generated code
    renoir_code.into()
}