use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Ident, Lit, MetaList, NestedMeta, Token, Type};

#[proc_macro]
pub fn try_as(args: TokenStream) -> TokenStream {
    let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
    let args: Vec<Ident> = parser.parse(args).unwrap().into_iter().collect();
    if args.len() != 2 {
        panic!("Must have exactly two arguments");
    }

    let value = &args[0];
    let target_type = &args[1];
    let target_method = format_ident!("as_{}", target_type);
    let target_type_name = target_type.to_string();
    quote! {
        #value
            .#target_method()
            .ok_or_else(|| anyhow!("Expected a {}, but found: {:#?}",
                #target_type_name, #value))
    }
    .into()
}
