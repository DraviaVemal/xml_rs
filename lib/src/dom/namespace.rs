// Copyright (c) DraviaVemal 2025
// Licensed under the Sponsorware License v4.0+ (see LICENSE for details).

use std::collections::HashMap;

use crate::XmlAttribute;

#[derive(Debug)]
pub struct XmlNamespace {
    url_alias: HashMap<String, String>,
    alias_url: HashMap<String, String>,
}

impl XmlNamespace {
    /// Create a new, empty namespace context.
    ///
    /// # Returns
    /// * `XmlNamespace` - An empty namespace context.
    pub fn new() -> Self {
        XmlNamespace {
            url_alias: HashMap::new(),
            alias_url: HashMap::new(),
        }
    }

    /// Add a mapping from alias to URL and vice versa.
    ///
    /// # Arguments
    /// * `alias` - The namespace alias (prefix).
    /// * `url` - The namespace URI.
    pub fn add_url_alias(&mut self, alias: String, url: String) {
        // Insert both directions for fast lookup
        self.url_alias.insert(alias.clone(), url.clone());
        self.alias_url.insert(url, alias);
    }

    /// Get the namespace URL for a given alias.
    ///
    /// # Arguments
    /// * `alias` - The namespace alias.
    ///
    /// # Returns
    /// * `Option<&String>` - The namespace URL if found.
    pub fn get_url(&self, alias: &str) -> Option<&String> {
        self.url_alias.get(alias)
    }

    /// Get the namespace alias for a given URL.
    ///
    /// # Arguments
    /// * `url` - The namespace URL.
    ///
    /// # Returns
    /// * `Option<&String>` - The alias if found.
    pub fn get_alias(&self, url: &str) -> Option<&String> {
        self.alias_url.get(url)
    }

    /// Add a namespace from an XmlAttribute (usually an xmlns attribute).
    ///
    /// # Arguments
    /// * `ns_attribute` - The attribute representing the namespace.
    pub fn add_namespace(&mut self, ns_attribute: XmlAttribute) {
        let ns_name = ns_attribute.get_ns_name();
        let url = ns_attribute.get_value().to_string();
        self.add_url_alias(ns_name, url);
    }
}
