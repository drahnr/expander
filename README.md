# expander

Expands a proc-macro into a file, and uses a `include!` directive in place.


## Usage

In your `proc-macro`, use it like:

```rust

#[proc_macro_attribute]
pub fn baz(_attr: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // wrap as per usual for `proc-macro2::TokenStream`, here dropping `attr` for simplicity
    baz2(input.into()).into()
}


 // or any other macro type
fn baz2(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let modified = quote::quote!{
        #[derive(Debug, Clone, Copy)]
        struct X {
            y: [u8:32],
        }
    };

    let expanded = Expander::new("baz.rs")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        // common way of gating this, by making it part of the default feature set
        .dry(cfg!(feature="no-file-expansion"))
        .write_to_out_dir(modified.clone()).unwrap_or_else(|e| {
            eprintln!("Failed to write to file: {:?}", e);
            modified
        });
    expanded
}
```

will expand into

```rust
include!("/absolute/path/to/your/project/target/debug/build/expander-49db7ae3a501e9f4/out/baz.rs");
```

where the file content will be

```rust
#[derive(Debug, Clone, Copy)]
struct X {
    y: [u8:32],
}
```

## Advantages

* Only expands a particular proc-macro, not all of them. I.e. `tracing` is notorious for expanding into a significant amount of boilerplate with i.e. `cargo expand`
* Get good errors when _your_ generated code is not perfect yet
