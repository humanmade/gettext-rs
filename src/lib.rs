//! The `gettext` crate provides functionality for
//! parsing and using Gettext catalogs stored in MO files.

// https://pascalhertleif.de/artikel/good-practices-for-writing-rust-libraries/
#![deny(missing_docs, missing_debug_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code, unstable_features,
        unused_import_braces, unused_qualifications)]

use std::collections::HashMap;
use std::ops::Deref;

/// Catalog represents a set of translation strings
/// parsed out of one MO file.
#[derive(Debug)]
pub struct Catalog {
    strings: HashMap<String, Message>,
}

impl Catalog {
    fn new() -> Self {
        Catalog { strings: HashMap::new() }
    }
    fn insert(&mut self, msg: Message) {
        let key = match msg.context {
            Some(ref ctxt) => [ctxt.deref(), &*msg.id].join("\x04"),
            None => msg.id.clone(),
        };
        self.strings.insert(key, msg);
    }
}

#[derive(Debug)]
struct Message {
    id: String,
    context: Option<String>,
    plural: Option<String>,
    translated: Vec<String>,
}

impl Message {
    fn new<T: Into<String>>(id: T,
                            context: Option<T>,
                            plural: Option<T>,
                            translated: Vec<T>)
                            -> Self {
        Message {
            id: id.into(),
            context: context.map(Into::into),
            plural: plural.map(Into::into),
            translated: translated.into_iter().map(Into::into).collect(),
        }
    }
}

#[test]
fn catalog_insert() {
    let mut cat = Catalog::new();
    println!("{:?}", cat.strings);
    cat.insert(Message::new("thisisid", None, None, vec![]));
    cat.insert(Message::new("anotherid", Some("context"), None, vec![]));
    let mut keys = cat.strings.keys().collect::<Vec<_>>();
    keys.sort();
    assert_eq!(keys, &["context\x04anotherid", "thisisid"])
}
