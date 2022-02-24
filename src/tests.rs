use super::*;

#[test]
fn dry() -> Result<(), std::io::Error> {
    let ts = quote! {
        pub struct X {
            x: [u8;32],
        }
    };
    let modified = Expander::new("foo")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        .dry(true)
        .write_to_out_dir(ts.clone())?;

    assert_eq!(
        ts.to_string(),
        modified.to_string(),
        "Dry does not alter the provided `TokenStream`. qed"
    );
    Ok(())
}

#[test]
fn basic() -> Result<(), std::io::Error> {
    let ts = quote! {
        pub struct X {
            x: [u8;32],
        }
    };
    let modified = Expander::new("bar")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        // .dry(false)
        .write_to_out_dir(ts.clone())?;

    let s = modified.to_string();
    assert_ne!(s, ts.to_string());
    assert!(s.contains("include ! ("));
    Ok(())
}

#[cfg(feature = "syndicate")]
mod syndicate {
    use super::*;
    use proc_macro2::Span;

    #[test]
    fn ok_is_written_to_external_file() -> Result<(), std::io::Error> {
        let ts = Ok(quote! {
            pub struct X {
                x: [u8;32],
            }
        });
        let modified = Expander::new("bar")
            .add_comment("This is generated code!".to_owned())
            .fmt(Edition::_2021)
            // .dry(false)
            .maybe_write_to_out_dir(ts.clone())?;

        let s = modified.to_string();
        assert_ne!(s, ts.unwrap().to_string());
        assert!(s.contains("include ! ("));
        Ok(())
    }

    #[test]
    fn errors_are_not_written_to_external_file() -> Result<(), std::io::Error> {
        let ts = Err(syn::Error::new(Span::call_site(), "Hajajajaiii!"));
        let modified = Expander::new("")
            .add_comment("This is generated code!".to_owned())
            .fmt(Edition::_2021)
            // .dry(false)
            .maybe_write_to_out_dir(ts.clone())?;

        assert_eq!(
            ts.unwrap_err().to_compile_error().to_string(),
            modified.to_string()
        );
        Ok(())
    }
}
