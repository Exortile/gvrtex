use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, DeriveInput, LitInt, Token};

struct BlockInput {
    x_block: LitInt,
    y_block: LitInt,
}

impl Parse for BlockInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let x_block = input.parse()?;
        let _comma: Token![,] = input.parse()?;
        let y_block = input.parse()?;
        Ok(Self { x_block, y_block })
    }
}

#[proc_macro_attribute]
pub fn gvr_encoder_base(attr: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let attr = parse_macro_input!(attr as BlockInput);

    // Get the struct name
    let name = &input.ident;
    let x_block = &attr.x_block;
    let y_block = &attr.y_block;

    let expanded = quote! {
        #input

        impl GvrBase for #name {
            fn get_block_size(&self) -> (u32, u32) {
                (#x_block, #y_block)
            }
        }

        impl GvrEncoderBase for #name {}
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn gvr_decoder_base(attr: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let attr = parse_macro_input!(attr as BlockInput);

    // Get the struct name
    let name = &input.ident;
    let x_block = &attr.x_block;
    let y_block = &attr.y_block;

    let expanded = quote! {
        #input

        impl GvrBase for #name {
            fn get_block_size(&self) -> (u32, u32) {
                (#x_block, #y_block)
            }
        }
    };

    TokenStream::from(expanded)
}
