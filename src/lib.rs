use fs_err as fs;
use proc_macro2::TokenStream;
use quote::quote;
use std::env;
use std::io::Write;
use std::path::Path;

/// Rust edition to format for.
#[derive(Debug, Clone, Copy)]
pub enum EditionArg {
    Unspecified,
    _2015,
    _2018,
    _2021,
}

impl std::default::Default for EditionArg {
    fn default() -> Self {
        Self::Unspecified
    }
}

impl std::fmt::Display for EditionArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::_2015 => "--edition 2015",
            Self::_2018 => "--edition 2018",
            Self::_2021 => "--edition 2021",
            Self::Unspecified => "",
        };
        write!(f, "{}", s)
    }
}

/// Expander to replace a tokenstream by a include to a file
#[derive(Default, Debug)]
pub struct Expander {
    /// Determines if the whole file `include!` should be done (`false`) or not (`true`).
    dry: bool,
    /// Filename for the generated indirection file to be used.
    filename: String,
    /// Additional comment to be added.
    comment: Option<String>,
    /// Format using `rustfmt` in your path.
    fmt: bool,
    /// Use provided edition formatting.
    fmt_edition_arg: EditionArg,
}

impl Expander {
    /// Create a new expander.
    pub fn new(name: impl AsRef<str>) -> Self {
        Self {
            dry: true,
            filename: name.as_ref().to_owned(),
            comment: None,
            fmt: false,
            fmt_edition_arg: EditionArg::default(),
        }
    }

    /// Add a header comment.
    pub fn add_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.comment = comment.into().map(|comment| format!("/* {} */\n", comment));
        self
    }

    /// Format the resulting file, for readability.
    pub fn fmt(mut self, edition: EditionArg) -> Self {
        self.fmt_edition_arg = edition;
        self.fmt = true;
        self
    }

    /// Set the filname to be placed under `OUT_DIR`.
    pub fn filename(mut self, name: impl AsRef<str>) -> Self {
        self.filename = name.as_ref().to_owned();
        self
    }

    /// Do not modify the provided tokenstream.
    pub fn dry(mut self, dry: bool) -> Self {
        self.dry = dry;
        self
    }

    /// Create a file with `filename` under `env!("OUT_DIR")`.
    pub fn write_to_out_dir(
        self,
        tokens: TokenStream,
        filename: &str,
    ) -> Result<TokenStream, std::io::Error> {
        if self.dry {
            Ok(tokens)
        } else {
            let out = env!("OUT_DIR");
            let out = std::path::PathBuf::from(out);
            let path = out.join(filename);
            expand_to_file(
                tokens,
                path.as_path(),
                out.as_path(),
                self.fmt,
                self.fmt_edition_arg,
                self.comment,
            )
        }
    }

    /// Create a file with `filename` at `dest`.
    pub fn write_to(self, tokens: TokenStream, dest: &Path) -> Result<TokenStream, std::io::Error> {
        if self.dry {
            Ok(tokens)
        } else {
            expand_to_file(
                tokens,
                dest,
                dest.parent().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Provided `dest` must be a file, and hence a parent must exist",
                    )
                })?,
                self.fmt,
                self.fmt_edition_arg,
                self.comment,
            )
        }
    }
}

/// Expand a proc-macro to file.
fn expand_to_file(
    tokens: TokenStream,
    dest: &Path,
    cwd: &Path,
    rustfmt: bool,
    edition: EditionArg,
    comment: impl Into<Option<String>>,
) -> Result<TokenStream, std::io::Error> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&dest)?;

    if let Some(comment) = comment.into() {
        f.write_all(&mut comment.as_bytes())?;
    }

    f.write_all(&mut tokens.to_string().as_bytes())?;

    if rustfmt {
        std::process::Command::new("rustfmt")
            .arg(edition.to_string())
            .arg(&dest)
            .current_dir(cwd)
            .spawn()?;
    }

    let dest = dest.display().to_string();
    Ok(quote! {
        include!( #dest );
    })
}
