use fs_err as fs;
use proc_macro2::TokenStream;
use quote::quote;
use std::env;
use std::io::Write;
use std::path::Path;

/// Rust edition to format for.
#[derive(Debug, Clone, Copy)]
pub enum Edition {
    Unspecified,
    _2015,
    _2018,
    _2021,
}

impl std::default::Default for Edition {
    fn default() -> Self {
        Self::Unspecified
    }
}

impl std::fmt::Display for Edition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::_2015 => "2015",
            Self::_2018 => "2018",
            Self::_2021 => "2021",
            Self::Unspecified => "",
        };
        write!(f, "{}", s)
    }
}

/// The channel to use for formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Channel {
    #[default]
    Default,
    Stable,
    Beta,
    Nightly,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Stable => "+stable",
            Self::Beta => "+beta",
            Self::Nightly => "+nightly",
            Self::Default => return Ok(()),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
enum RustFmt {
    Yes {
        edition: Edition,
        channel: Channel,
        allow_failure: bool,
    },
    No,
}

impl std::default::Default for RustFmt {
    fn default() -> Self {
        RustFmt::No
    }
}

impl From<Edition> for RustFmt {
    fn from(edition: Edition) -> Self {
        RustFmt::Yes {
            edition,
            channel: Channel::Default,
            allow_failure: false,
        }
    }
}

/// Expander to replace a tokenstream by a include to a file
#[derive(Default, Debug)]
pub struct Expander {
    /// Determines if the whole file `include!` should be done (`false`) or not (`true`).
    dry: bool,
    /// If `true`, print the generated destination file to terminal.
    verbose: bool,
    /// Filename for the generated indirection file to be used.
    filename_base: String,
    /// Additional comment to be added.
    comment: Option<String>,
    /// Format using `rustfmt` in your path.
    rustfmt: RustFmt,
}

impl Expander {
    /// Create a new expander.
    ///
    /// The `filename_base` will be expanded to `{filename_base}-{digest}.rs` in order to dismabiguate
    /// .
    pub fn new(filename_base: impl AsRef<str>) -> Self {
        Self {
            dry: false,
            verbose: false,
            filename_base: filename_base.as_ref().to_owned(),
            comment: None,
            rustfmt: RustFmt::No,
        }
    }

    /// Add a header comment.
    pub fn add_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.comment = comment.into().map(|comment| format!("/* {} */\n", comment));
        self
    }

    /// Format the resulting file, for readability.
    pub fn fmt(mut self, edition: impl Into<Edition>) -> Self {
        self.rustfmt = RustFmt::Yes {
            edition: edition.into(),
            channel: Channel::Default,
            allow_failure: false,
        };
        self
    }

    /// Format the resulting file, for readability.
    ///
    /// Allows to specify `channel` and if a failure is fatal in addition.
    ///
    /// Note: Calling [`fn fmt(..)`] afterwards will override settings given.
    pub fn fmt_full(
        mut self,
        channel: impl Into<Channel>,
        edition: impl Into<Edition>,
        allow_failure: bool,
    ) -> Self {
        self.rustfmt = RustFmt::Yes {
            edition: edition.into(),
            channel: channel.into(),
            allow_failure,
        };
        self
    }

    /// Do not modify the provided tokenstream.
    pub fn dry(mut self, dry: bool) -> Self {
        self.dry = dry;
        self
    }

    /// Print the path of the generated file to `stderr` during the proc-macro invocation.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    #[cfg(any(feature = "syndicate", test))]
    /// Create a file with `filename` under `env!("OUT_DIR")` if it's not an `Err(_)`.
    pub fn maybe_write_to_out_dir(
        self,
        tokens: impl Into<Result<TokenStream, syn::Error>>,
    ) -> Result<syn::Result<TokenStream>, std::io::Error> {
        self.maybe_write_to(tokens, std::path::PathBuf::from(env!("OUT_DIR")).as_path())
    }

    /// Create a file with `filename` under `env!("OUT_DIR")`.
    pub fn write_to_out_dir(self, tokens: TokenStream) -> Result<TokenStream, std::io::Error> {
        let out = std::path::PathBuf::from(env!("OUT_DIR"));
        self.write_to(tokens, out.as_path())
    }

    #[cfg(any(feature = "syndicate", test))]
    /// Create a file with `filename` at `dest` if it's not an `Err(_)`.
    pub fn maybe_write_to(
        self,
        maybe_tokens: impl Into<Result<TokenStream, syn::Error>>,
        dest_dir: &Path,
    ) -> Result<syn::Result<TokenStream>, std::io::Error> {
        match maybe_tokens.into() {
            Ok(tokens) => Ok(Ok(self.write_to(tokens, dest_dir)?)),
            err => Ok(err),
        }
    }

    /// Create a file with `self.filename` in  `dest_dir`.
    pub fn write_to(
        self,
        tokens: TokenStream,
        dest_dir: &Path,
    ) -> Result<TokenStream, std::io::Error> {
        if self.dry {
            Ok(tokens)
        } else {
            expand_to_file(
                tokens,
                dest_dir.join(self.filename_base).as_path(),
                dest_dir,
                self.rustfmt,
                self.comment,
                self.verbose,
            )
        }
    }
}

/// Take the leading 6 bytes and convert them to 12 hex ascii characters.
fn make_suffix(digest: &[u8; 32]) -> String {
    let mut shortened_hex = String::with_capacity(12);
    const TABLE: &[u8] = b"0123456789abcdef";
    for &byte in digest.iter().take(6) {
        shortened_hex.push(TABLE[((byte >> 4) & 0x0F) as usize] as char);
        shortened_hex.push(TABLE[((byte >> 0) & 0x0F) as usize] as char);
    }
    shortened_hex
}

/// Expand a proc-macro to file.
///
/// The current working directory `cwd` is only used for the `rustfmt` invocation
/// and hence influences where the config files would be pulled in from.
fn expand_to_file(
    tokens: TokenStream,
    dest: &Path,
    cwd: &Path,
    rustfmt: RustFmt,
    comment: impl Into<Option<String>>,
    verbose: bool,
) -> Result<TokenStream, std::io::Error> {
    let token_str = tokens.to_string();
    #[cfg(feature = "pretty")]
    let token_str = match syn::parse_file(&token_str) {
        Err(e) => {
            eprintln!("expander: failed to prettify {}: {:?}", dest.display(), e);
            token_str
        }
        Ok(sf) => prettyplease::unparse(&sf),
    };

    // we need to disambiguate for transitive dependencies, that might create different output to not override one another
    let mut bytes = token_str.as_bytes();
    let hash = <blake2::Blake2s256 as blake2::Digest>::digest(bytes);
    let shortened_hex = make_suffix(hash.as_ref());

    let dest =
        std::path::PathBuf::from(dest.display().to_string() + "-" + shortened_hex.as_str() + ".rs");

    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest.as_path())?;

    // Force implicit drop of FileGuard before rustfmt to prevent intermittent partial file locking issues on Windows (os error 33)
    {
        let Ok(mut f) = file_guard::try_lock(f.file_mut(), file_guard::Lock::Exclusive, 0, 64)
        else {
            // the digest of the file will not match if the content to be written differed, hence any existing lock
            // means we are already writing the same content to the file
            if verbose {
                eprintln!("expander: already in progress of writing identical content to {} by a different crate", dest.display());
            }
            // now actually wait until the write is complete
            let _lock = file_guard::lock(f.file_mut(), file_guard::Lock::Exclusive, 0, 64)
                .expect("File Lock never fails us. qed");

            if verbose {
                eprintln!("expander: lock was release, referencing");
            }

            let dest = dest.display().to_string();
            return Ok(quote! {
                include!( #dest );
            });
        };

        if verbose {
            eprintln!("expander: writing {}", dest.display());
        }

        if let Some(comment) = comment.into() {
            f.write_all(&mut comment.as_bytes())?;
        }

        f.write_all(&mut bytes)?;
    } // guard implicitly dropped here; now safe to call rustfmt

    if let RustFmt::Yes {
        channel,
        edition,
        allow_failure,
    } = rustfmt
    {
        let mut process = std::process::Command::new("rustfmt");
        if Channel::Default != channel {
            process.arg(channel.to_string());
        }
        process
            .arg(format!("--edition={}", edition))
            .arg(&dest)
            .current_dir(cwd);

        handle_rustfmt_result(process.status(), &dest, allow_failure)?;
    }

    let dest = dest.display().to_string();
    Ok(quote! {
        include!( #dest );
    })
}

fn handle_rustfmt_result(
    res: std::io::Result<std::process::ExitStatus>,
    dest: &Path,
    allow_failure: bool,
) -> std::io::Result<()> {
    match res {
        Err(err) => {
            if allow_failure {
                eprintln!(
                    "expander: failed to format file {} due to {}",
                    dest.display(),
                    err
                );
                Ok(())
            } else {
                Err(err)
            }
        }
        Ok(exit_status) => {
            if !exit_status.success() {
                let error = std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "rustfmt failed with exit code {} on file {}",
                        exit_status.code().unwrap_or(-1),
                        dest.display()
                    ),
                );
                if allow_failure {
                    eprintln!("expander: {}", error);
                    Ok(())
                } else {
                    Err(error)
                }
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests;
