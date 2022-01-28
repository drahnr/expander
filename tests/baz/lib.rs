use expander::{Expander, Edition};

#[proc_macro_attribute]
pub fn baz(_attr: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    baz2(input.into()).into()
}

fn baz2(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let modified = quote::quote!{
        #[derive(Debug, Clone, Copy)]
        #input
    };

    let expanded = Expander::new("baz.rs")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        .write_to_out_dir(modified).expect("IO error");
    expanded
}
