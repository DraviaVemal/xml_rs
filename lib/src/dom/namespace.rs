/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::XmlAttribute;
use std::collections::HashMap;

/// Namespace alias key
pub type NsAlias = String;
/// Namespace url
pub type NsUrl = String;

/// Manages XML namespace mappings between aliases (prefixes) and URLs.
///
/// This struct provides bidirectional mapping between namespace prefixes and their
/// corresponding URLs, allowing for efficient lookups in both directions.
#[derive(Debug)]
pub struct XmlNamespace {
    /// Maps from namespace alias to URL
    url_alias: HashMap<NsUrl, NsAlias>,
    /// Maps from namespace URL to alias
    alias_url: HashMap<NsAlias, NsUrl>,
}

impl XmlNamespace {
    // --------------------------
    // pub(crate) mut self methods
    // --------------------------

    /// Adds a mapping from alias to URL and vice versa.
    ///
    /// # Arguments
    /// * `alias` - The namespace alias (prefix).
    /// * `url` - The namespace URI.
    pub(crate) fn add_url_alias_mut(&mut self, alias: &str, url: &str) {
        // Insert both directions for bidirectional lookup capability
        self.alias_url.insert(alias.to_owned(), url.to_owned());
        self.url_alias.insert(url.to_owned(), alias.to_owned());
    }

    /// Adds a namespace from an XML attribute (usually an xmlns attribute).
    ///
    /// # Arguments
    /// * `ns_attribute` - The attribute representing the namespace declaration.
    pub(crate) fn add_namespace_mut(&mut self, ns_attribute: XmlAttribute) {
        // Extract the namespace name and URL from the attribute
        let ns_name = ns_attribute
            .get_ns_name()
            .split(":")
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
            .get(1)
            .cloned()
            .unwrap_or_default();
        let url = ns_attribute.get_value();

        // Add the mapping
        self.add_url_alias_mut(&ns_name, url);
    }
}

impl XmlNamespace {
    // --------------------------
    // pub(crate) self methods
    // --------------------------

    /// Gets the namespace URL for a given alias.
    ///
    /// # Arguments
    /// * `alias` - The namespace alias/prefix.
    ///
    /// # Returns
    /// * `Option<&String>` - The namespace URL if the alias is found, None otherwise.
    pub(crate) fn _get_url(&self, alias: &str) -> Option<&String> {
        self.alias_url.get(alias)
    }

    /// Gets the namespace alias for a given URL.
    ///
    /// # Arguments
    /// * `url` - The namespace URL.
    ///
    /// # Returns
    /// * `Option<&String>` - The alias/prefix if the URL is found, None otherwise.
    pub(crate) fn _get_alias(&self, url: &str) -> Option<&String> {
        self.url_alias.get(url)
    }

    /// Returns a reference to the namespace context for this element.
    pub(crate) fn get_namespace_alias_url(&self) -> &HashMap<NsAlias, NsUrl> {
        &self.alias_url
    }
}

impl XmlNamespace {
    // --------------------------
    // pub(crate) constructor
    // --------------------------

    /// Creates a new, empty namespace context.
    ///
    /// # Returns
    /// * `XmlNamespace` - An empty namespace context with no mappings.
    pub(crate) fn new() -> Self {
        XmlNamespace {
            url_alias: HashMap::new(),
            alias_url: HashMap::new(),
        }
    }
}
