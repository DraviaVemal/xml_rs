/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::XmlAttribute;
use log::{debug, trace, warn};
use std::collections::HashMap;

/// Namespace alias key
pub type NsAlias = String;
/// Namespace url
pub type NsUrl = String;

/// Manages XML namespace mappings between aliases (prefixes) and URLs.
///
/// This struct provides bidirectional mapping between namespace prefixes and their
/// corresponding URLs, allowing for efficient lookups in both directions.
#[derive(Debug, Clone)]
pub struct XmlNamespace {
    /// Maps from namespace alias to URL
    url_alias: HashMap<NsUrl, NsAlias>,
    /// Maps from namespace URL to alias
    alias_url: HashMap<NsAlias, (NsUrl, u32)>,
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
        // When alias is rebound to a new URL, remove the stale url→alias entry first
        if let Some((old_url, _)) = self.alias_url.get(alias) {
            if old_url != url {
                warn!(
                    "draviavemal-xml_rs::Namespace alias '{}' rebound from '{}' to '{}'",
                    alias, old_url, url
                );
                self.url_alias.remove(old_url.as_str());
            }
        }
        trace!(
            "draviavemal-xml_rs::Registered namespace alias '{}' -> '{}'",
            alias,
            url
        );
        self.alias_url.insert(alias.to_owned(), (url.to_owned(), 0));
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

        if ns_name.is_empty() {
            debug!(
                "draviavemal-xml_rs::Registering default namespace -> '{}'",
                url
            );
        } else {
            debug!(
                "draviavemal-xml_rs::Registering namespace '{}' -> '{}'",
                ns_name, url
            );
        }

        // Add the mapping
        self.add_url_alias_mut(&ns_name, url);
    }

    pub(crate) fn is_valid_ns_alias(&self, ns_alias: &str) -> bool {
        self.alias_url.contains_key(ns_alias)
    }

    /// Increments the usage counter for the given alias, ignoring unknown aliases.
    pub(crate) fn increment_alias_use_mut(&mut self, alias: &str) {
        if let Some((_, usage_count)) = self.alias_url.get_mut(alias) {
            *usage_count += 1;
        }
    }

    /// Decrements the usage counter for the given alias, saturating at zero.
    pub(crate) fn decrement_alias_use_mut(&mut self, alias: &str) {
        if let Some((_, usage_count)) = self.alias_url.get_mut(alias) {
            *usage_count = usage_count.saturating_sub(1);
        }
    }

    /// Resets every alias usage counter in this scope to zero.
    pub(crate) fn reset_alias_use_mut(&mut self) {
        for (_, (_, usage_count)) in self.alias_url.iter_mut() {
            *usage_count = 0;
        }
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
    pub(crate) fn _get_url(&self, alias: &str) -> Option<&(NsUrl, u32)> {
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
    pub(crate) fn get_namespace_alias_url(&self) -> &HashMap<NsAlias, (NsUrl, u32)> {
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
