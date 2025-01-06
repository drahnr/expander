use super::*;
use proc_macro2::Span;
use tempfile::TempDir;

fn setup_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

fn create_test_content(content: &str) -> Vec<u8> {
    content.as_bytes().to_vec()
}

// Helper function to normalize line endings
fn normalize_line_endings(s: &str) -> String {
    if cfg!(windows) {
        s.replace("\r\n", "\n")
    } else {
        s.to_string()
    }
}

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

#[test]
fn syn_ok_is_written_to_external_file() -> Result<(), std::io::Error> {
    let ts = Ok(quote! {
        pub struct X {
            x: [u8;32],
        }
    });
    let result = Expander::new("bar")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        // .dry(false)
        .maybe_write_to_out_dir(ts.clone())?;
    let modified = result.expect("Is not a syn error. qed");

    let s = modified.to_string();
    assert_ne!(s, ts.unwrap().to_string());
    assert!(s.contains("include ! "));
    Ok(())
}

#[test]
fn syn_error_is_not_written_to_external_file() -> Result<(), std::io::Error> {
    const MSG: &str = "Hajajajaiii!";
    let ts = Err(syn::Error::new(Span::call_site(), MSG));
    let result = Expander::new("")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        // .dry(false)
        .maybe_write_to_out_dir(ts.clone())?;
    let modified = result.expect_err("Is a syn error. qed");

    let s = modified.to_compile_error().to_string();
    assert!(dbg!(&s).contains("compile_error !"));
    assert!(s.contains(MSG));

    Ok(())
}

#[test]
fn test_basic_formatting() {
    let input = create_test_content("struct Foo{x:i32,y:String}");
    let result = run_rustfmt_on_content(&input, Channel::Default, Edition::_2021, false)
        .expect("Formatting failed");

    let formatted = normalize_line_endings(&String::from_utf8(result).expect("Invalid UTF-8"));
    assert!(formatted.contains("struct Foo {\n"));
    assert!(formatted.contains("    x: i32,\n"));
    assert!(formatted.contains("    y: String,\n"));
    assert!(formatted.contains("}\n"));
}

#[test]
fn test_formatting_with_comments() {
    let input = create_test_content("// Comment\nstruct Foo{x:i32} // Inline comment");
    let result = run_rustfmt_on_content(&input, Channel::Default, Edition::_2021, false)
        .expect("Formatting failed");

    let formatted = String::from_utf8(result).expect("Invalid UTF-8");
    assert!(formatted.contains("// Comment\n"));
    assert!(formatted.contains("// Inline comment\n"));
}

#[test]
fn test_complete_expansion() {
    let temp_dir = setup_test_dir();
    let dest = temp_dir.path().join("test.rs");

    let tokens = quote::quote! {
        struct Test {
            field: i32
        }
    };

    let _result = expand_to_file(
        tokens.into(),
        &dest,
        temp_dir.path(),
        RustFmt::Yes {
            channel: Channel::Default,
            edition: Edition::_2021,
            allow_failure: false,
        },
        Some("/* Test */".to_string()),
        true,
    )
    .expect("Expansion failed");

    // Find the generated file (it will have a hash suffix)
    let generated_file = fs::read_dir(temp_dir.path())
        .expect("Failed to read temp dir")
        .filter_map(Result::ok)
        .find(|entry| entry.file_name().to_string_lossy().starts_with("test.rs-"))
        .expect("Generated file not found");

    // Check the generated file exists and contains expected content
    let content = fs::read_to_string(generated_file.path()).expect("Failed to read generated file");
    assert!(content.contains("/* Test */"));
    assert!(content.contains("struct Test"));
}

#[test]
fn test_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = setup_test_dir();
    let temp_dir = Arc::new(temp_dir);

    // Test concurrent formatting of different content
    let handles: Vec<_> = (0..3)
        .map(|i| {
            let _temp_dir = Arc::clone(&temp_dir);

            thread::spawn(move || {
                let content = format!("struct Test_{} {{ field: i32 }}", i); // Use underscore in name
                run_rustfmt_on_content(content.as_bytes(), Channel::Default, Edition::_2021, false)
            })
        })
        .collect();

    // Verify all formatting operations completed successfully
    for handle in handles {
        handle.join().unwrap().expect("Thread operation failed");
    }
}

#[test]
fn test_formatting_errors() {
    let input = create_test_content("struct Invalid { missing_semicolon }"); // More realistic invalid Rust
    let result = run_rustfmt_on_content(
        &input,
        Channel::Default,
        Edition::_2021,
        true, // allow_failure
    );

    assert!(result.is_ok(), "Should not fail when allow_failure is true");
    assert_eq!(
        result.unwrap(),
        input,
        "Should return original content when formatting fails with allow_failure=true"
    );

    let result = run_rustfmt_on_content(
        &input,
        Channel::Default,
        Edition::_2021,
        false, // don't allow failure
    );

    assert!(result.is_err(), "Should fail when allow_failure is false");
    assert!(
        result.unwrap_err().to_string().contains("rustfmt failed"),
        "Error should mention rustfmt failure"
    );
}

#[test]
fn test_large_file() {
    // Test with a larger file (100KB of valid Rust code)
    let mut content = String::with_capacity(102400);
    for i in 0..1000 {
        content.push_str(&format!("struct Large{} {{ field: i32 }}\n", i));
    }

    let result =
        run_rustfmt_on_content(content.as_bytes(), Channel::Default, Edition::_2021, false)
            .expect("Formatting large file failed");

    assert!(
        result.len() > content.len(),
        "Formatted content should include proper spacing"
    );
}

#[test]
#[cfg(not(feature = "pretty"))]
fn test_maybe_rustfmt_without_pretty_feature() {
    // Test with rustfmt enabled
    let rustfmt = RustFmt::Yes {
        channel: Channel::Default,
        edition: Edition::_2021,
        allow_failure: false,
    };
    let input = "struct Foo{x:i32}".to_string();

    let result = maybe_run_rustfmt_on_content(
        &rustfmt,
        true,
        "test: expander: formatting with rustfmt",
        input.clone(),
    )
    .expect("Formatting failed");
    let formatted = normalize_line_endings(&String::from_utf8(result).expect("Invalid UTF-8"));
    assert!(formatted.contains("struct Foo {\n    x: i32,\n}"));

    // Test with rustfmt disabled
    let rustfmt = RustFmt::No;
    let result = maybe_run_rustfmt_on_content(
        &rustfmt,
        true,
        "test: expander: formatting with rustfmt",
        input.clone(),
    )
    .expect("Should return unformatted content");
    let unformatted = String::from_utf8(result).expect("Invalid UTF-8");
    assert_eq!(unformatted, input);
}

#[test]
#[cfg(feature = "pretty")]
fn test_maybe_rustfmt_with_pretty_feature_failure() {
    // Invalid Rust code that will fail syn::parse_file
    let input = "struct Foo { invalid rust".to_string();

    // Test with rustfmt enabled as fallback
    let rustfmt = RustFmt::Yes {
        channel: Channel::Default,
        edition: Edition::_2021,
        allow_failure: true,
    };

    let result = maybe_run_rustfmt_on_content(
        &rustfmt,
        true,
        "test: expander falling back to rustfmt because syn::parse failed, with allow_failure=true",
        input.clone(),
    )
    .expect("Should not fail with allow_failure=true");

    // With allow_failure=true, should get original content back
    assert_eq!(String::from_utf8(result).expect("Invalid UTF-8"), input);

    // Test with rustfmt disabled
    let rustfmt = RustFmt::No;
    let result = maybe_run_rustfmt_on_content(
        &rustfmt,
        true,
        "test: expander trying rustfmt because syn::parse failed but rustfmt not available",
        input.clone(),
    )
    .expect("Should return unformatted content");
    assert_eq!(String::from_utf8(result).expect("Invalid UTF-8"), input);
}

#[test]
#[cfg(feature = "pretty")]
fn test_maybe_rustfmt_with_pretty_feature_failure_strict() {
    // Invalid Rust code that will fail syn::parse_file
    let input = "struct Foo { invalid rust".to_string();

    // Test with rustfmt enabled as fallback and allow_failure=false
    let rustfmt = RustFmt::Yes {
        channel: Channel::Default,
        edition: Edition::_2021,
        allow_failure: false,
    };

    let result = maybe_run_rustfmt_on_content(&rustfmt, true, "test: expander falling back to rustfmt because syn::parse failed, with allow_failure=false", input);
    assert!(result.is_err(), "Should fail with allow_failure=false");
    assert!(result.unwrap_err().to_string().contains("rustfmt failed"));
}
