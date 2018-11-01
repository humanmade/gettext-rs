//! This crate is a reimplementation
//! of GNU gettext translation framework in Rust.
//! It allows your Rust programs to parse out GNU MO files
//! containing translations and use them in your user interface.
//!
//! It contains several differences from the official C implementation.
//! Notably, this crate does not in any way depend on a global locale
//! ([2.2](https://www.gnu.org/software/gettext/manual/gettext.html#Setting-the-GUI-Locale))
//! and does not enforce a directory structure
//! for storing your translation catalogs
//! ([11.2.3](https://www.gnu.org/software/gettext/manual/gettext.html#Locating-Catalogs)).
//! Instead, the choice of translation catalog to use is explicitly made by the user.
//!
//! This crate is still in-progress
//! and may not be on par with the original implementation feature-wise.
//!
//! For the exact feature parity see the roadmap in the
//! [README](https://github.com/justinas/gettext#readme).
//!
//! # Example
//!
//! ```ignore
//! extern crate gettext;
//!
//! use std::fs::File;
//! use gettext::Catalog;
//!
//! fn main() {
//!     let f = File::open("french.mo").expect("could not open the catalog");
//!     let catalog = Catalog::parse(f).expect("could not parse the catalog");
//!
//!     // Will print out the French translation
//!     // if it is found in the parsed file
//!     // or "Name" otherwise.
//!     println!("{}", catalog.gettext("Name"));
//! }
//! ```

// https://pascalhertleif.de/artikel/good-practices-for-writing-rust-libraries/
#![deny(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces
)]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

mod metadata;
mod parser;
mod plurals;

use std::collections::HashMap;
use std::io::Read;
use std::ops::Deref;

pub use parser::{default_resolver, Error, ParseOptions};
use plurals::*;

fn key_with_context(context: &str, key: &str) -> String {
    let mut result = context.to_owned();
    result.push('\x04');
    result.push_str(key);
    result
}

/// Catalog represents a set of translation strings
/// parsed out of one MO file.
#[derive(Clone, Debug)]
pub struct Catalog {
    strings: HashMap<String, Message>,
    resolver: Resolver,
}

impl Catalog {
    /// Creates a new, empty gettext catalog.
    fn new() -> Self {
        Catalog {
            strings: HashMap::new(),
            resolver: Resolver::Function(default_resolver),
        }
    }

    /// Parses a gettext catalog from the given binary MO file.
    /// Returns the `Err` variant upon encountering an invalid file format
    /// or invalid byte sequence in strings.
    ///
    /// Calling this method is equivalent to calling
    /// `ParseOptions::new().parse(reader)`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use gettext::Catalog;
    /// use std::fs::File;
    ///
    /// let file = File::open("french.mo").unwrap();
    /// let catalog = Catalog::parse(file).unwrap();
    /// ```
    pub fn parse<R: Read>(reader: R) -> Result<Self, parser::Error> {
        ParseOptions::new().parse(reader)
    }

    fn insert(&mut self, msg: Message) {
        let key = match msg.context {
            Some(ref ctxt) => key_with_context(ctxt, &msg.id),
            None => msg.id.clone(),
        };
        self.strings.insert(key, msg);
    }

    /// Returns the singular translation of `msg_id` from the given catalog
    /// or `msg_id` itself if a translation does not exist.
    pub fn gettext<'a>(&'a self, msg_id: &'a str) -> &'a str {
        self.strings
            .get(msg_id)
            .and_then(|msg| msg.get_translated(0))
            .unwrap_or(msg_id)
    }

    /// Returns the plural translation of `msg_id` from the given catalog
    /// with the correct plural form for the number `n` of objects.
    /// Returns msg_id if a translation does not exist and `n == 1`,
    /// msg_id_plural otherwise.
    ///
    /// Currently, the only supported plural formula is `n != 1`.
    pub fn ngettext<'a>(&'a self, msg_id: &'a str, msg_id_plural: &'a str, n: u64) -> &'a str {
        let form_no = self.resolver.resolve(n);

        match self.strings.get(msg_id) {
            Some(msg) => msg
                .get_translated(form_no)
                .unwrap_or_else(|| [msg_id, msg_id_plural][form_no]),
            None if n == 1 => msg_id,
            None if n != 1 => msg_id_plural,
            _ => unreachable!(),
        }
    }

    /// Returns the singular translation of `msg_id`
    /// in the context `msg_context`
    /// or `msg_id` itself if a translation does not exist.
    // TODO: DRY gettext/pgettext
    pub fn pgettext<'a>(&'a self, msg_context: &'a str, msg_id: &'a str) -> &'a str {
        let key = key_with_context(msg_context, &msg_id);
        self.strings
            .get(&key)
            .and_then(|msg| msg.get_translated(0))
            .unwrap_or(msg_id)
    }

    /// Returns the plural translation of `msg_id`
    /// in the context `msg_context`
    /// with the correct plural form for the number `n` of objects.
    /// Returns msg_id if a translation does not exist and `n == 1`,
    /// msg_id_plural otherwise.
    ///
    /// Currently, the only supported plural formula is `n != 1`.
    // TODO: DRY ngettext/npgettext
    pub fn npgettext<'a>(
        &'a self,
        msg_context: &'a str,
        msg_id: &'a str,
        msg_id_plural: &'a str,
        n: u64,
    ) -> &'a str {
        let key = key_with_context(msg_context, &msg_id);
        let form_no = self.resolver.resolve(n);
        match self.strings.get(&key) {
            Some(msg) => msg
                .get_translated(form_no)
                .unwrap_or_else(|| [msg_id, msg_id_plural][form_no]),
            None if n == 1 => msg_id,
            None if n != 1 => msg_id_plural,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Message {
    id: String,
    context: Option<String>,
    translated: Vec<String>,
}

impl Message {
    fn new<T: Into<String>>(id: T, context: Option<T>, translated: Vec<T>) -> Self {
        Message {
            id: id.into(),
            context: context.map(Into::into),
            translated: translated.into_iter().map(Into::into).collect(),
        }
    }

    fn get_translated(&self, form_no: usize) -> Option<&str> {
        self.translated.get(form_no).map(|s| s.deref())
    }
}

#[test]
fn catalog_insert() {
    let mut cat = Catalog::new();
    cat.insert(Message::new("thisisid", None, vec![]));
    cat.insert(Message::new("anotherid", Some("context"), vec![]));
    let mut keys = cat.strings.keys().collect::<Vec<_>>();
    keys.sort();
    assert_eq!(keys, &["context\x04anotherid", "thisisid"])
}

#[test]
fn catalog_gettext() {
    let mut cat = Catalog::new();
    cat.insert(Message::new("Text", None, vec!["Tekstas"]));
    cat.insert(Message::new("Image", Some("context"), vec!["Paveikslelis"]));
    assert_eq!(cat.gettext("Text"), "Tekstas");
    assert_eq!(cat.gettext("Image"), "Image");
}

#[test]
fn catalog_ngettext() {
    let mut cat = Catalog::new();
    {
        // n == 1, no translation
        assert_eq!(cat.ngettext("Text", "Texts", 1), "Text");
        // n != 1, no translation
        assert_eq!(cat.ngettext("Text", "Texts", 0), "Texts");
        assert_eq!(cat.ngettext("Text", "Texts", 2), "Texts");
    }
    {
        cat.insert(Message::new("Text", None, vec!["Tekstas", "Tekstai"]));
        // n == 1, translation available
        assert_eq!(cat.ngettext("Text", "Texts", 1), "Tekstas");
        // n != 1, translation available
        assert_eq!(cat.ngettext("Text", "Texts", 0), "Tekstai");
        assert_eq!(cat.ngettext("Text", "Texts", 2), "Tekstai");
    }
}

#[test]
fn catalog_pgettext() {
    let mut cat = Catalog::new();
    cat.insert(Message::new("Text", Some("unit test"), vec!["Tekstas"]));
    assert_eq!(cat.pgettext("unit test", "Text"), "Tekstas");
    assert_eq!(cat.pgettext("integration test", "Text"), "Text");
}

#[test]
fn catalog_npgettext() {
    let mut cat = Catalog::new();
    cat.insert(Message::new(
        "Text",
        Some("unit test"),
        vec!["Tekstas", "Tekstai"],
    ));

    assert_eq!(cat.npgettext("unit test", "Text", "Texts", 1), "Tekstas");
    assert_eq!(cat.npgettext("unit test", "Text", "Texts", 0), "Tekstai");
    assert_eq!(cat.npgettext("unit test", "Text", "Texts", 2), "Tekstai");

    assert_eq!(
        cat.npgettext("integration test", "Text", "Texts", 1),
        "Text"
    );
    assert_eq!(
        cat.npgettext("integration test", "Text", "Texts", 0),
        "Texts"
    );
    assert_eq!(
        cat.npgettext("integration test", "Text", "Texts", 2),
        "Texts"
    );
}


#[test]
fn test_complex_plural() {
    let reader: &[u8] = include_bytes!("../test_cases/complex_plural.mo");
    let cat = parser::parse_catalog(reader, ParseOptions::new()).unwrap();

    assert_eq!(cat.ngettext("Test", "Tests", 0), "Plural 2");
    assert_eq!(cat.ngettext("Test", "Tests", 1), "Singular");
    assert_eq!(cat.ngettext("Test", "Tests", 2), "Plural 1");
    for i in 3..20 {
        assert_eq!(cat.ngettext("Test", "Tests", i), "Plural 2");
    }
}
