use expander::{Channel, Edition, Expander};

#[proc_macro_attribute]
pub fn baz(_attr: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    baz2(input.into()).into()
}

fn baz2(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let modified = quote::quote!{
        #[derive(Debug, Clone, Copy)]
        #input
    };

    let expanded = Expander::new("baz")
        .verbose(true)
        .add_comment("This is generated code!".to_owned())
        .fmt_full(Channel::Stable, Edition::_2021, true)
        .write_to_out_dir(modified).expect("No IO error happens. qed");
    expanded
}
