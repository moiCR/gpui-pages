use proc_macro::TokenStream;

#[proc_macro]
pub fn include_pages(input: TokenStream) -> TokenStream {
    let expansion = if input.is_empty() {
        r#"include!(concat!(env!("OUT_DIR"), "/pages.rs"));"#
    } else {
        r#"compile_error!("include_pages! takes no arguments");"#
    };
    expansion.parse().unwrap_or_default()
}
