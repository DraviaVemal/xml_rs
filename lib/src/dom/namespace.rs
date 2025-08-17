/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::XmlAttribute;
use std::collections::HashMap;

#[derive(Debug)]
pub struct XmlNamespace {
    url_alias: HashMap<String, String>,
    alias_url: HashMap<String, String>,
}

impl XmlNamespace {
    /// Add a mapping from alias to URL and vice versa.
    ///
    /// # Arguments
    /// * `alias` - The namespace alias (prefix).
    /// * `url` - The namespace URI.
    pub(crate) fn add_url_alias_mut(&mut self, alias: String, url: String) {
        // Insert both directions for fast lookup
        self.url_alias.insert(alias.clone(), url.clone());
        self.alias_url.insert(url, alias);
    }

    /// Add a namespace from an XmlAttribute (usually an xmlns attribute).
    ///
    /// # Arguments
    /// * `ns_attribute` - The attribute representing the namespace.
    pub(crate) fn add_namespace_mut(&mut self, ns_attribute: XmlAttribute) {
        let ns_name = ns_attribute.get_ns_name();
        let url = ns_attribute.get_value().to_string();
        self.add_url_alias_mut(ns_name, url);
    }
}

impl XmlNamespace {
    /// Get the namespace URL for a given alias.
    ///
    /// # Arguments
    /// * `alias` - The namespace alias.
    ///
    /// # Returns
    /// * `Option<&String>` - The namespace URL if found.
    pub(crate) fn get_url(&self, alias: &str) -> Option<&String> {
        self.url_alias.get(alias)
    }

    /// Get the namespace alias for a given URL.
    ///
    /// # Arguments
    /// * `url` - The namespace URL.
    ///
    /// # Returns
    /// * `Option<&String>` - The alias if found.
    pub(crate) fn get_alias(&self, url: &str) -> Option<&String> {
        self.alias_url.get(url)
    }
}

impl XmlNamespace {
    /// Create a new, empty namespace context.
    ///
    /// # Returns
    /// * `XmlNamespace` - An empty namespace context.
    pub(crate) fn new() -> Self {
        XmlNamespace {
            url_alias: HashMap::new(),
            alias_url: HashMap::new(),
        }
    }
}
