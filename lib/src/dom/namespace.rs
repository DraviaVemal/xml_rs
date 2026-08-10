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

/// Declarative namespace binding pairing a URI with its canonical alias.
///
/// Passed to the namespace-aware element and attribute APIs so alias mapping and
/// declaration are resolved against the live document scope instead of hard coded
/// prefixes. Resolution precedence is `alias_override` > alias already in scope for
/// `uri` > `default_alias`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamespaceDeclaration {
    /// W3C namespace URI, e.g. "http://schemas.openxmlformats.org/drawingml/2006/main".
    pub uri: &'static str,
    /// Alias used when the URI is not already declared in scope, e.g. "a".
    pub default_alias: &'static str,
    /// Forces this alias regardless of any alias already bound to the URI in scope.
    pub alias_override: Option<&'static str>,
}

impl NamespaceDeclaration {

    /// Builds a declaration that adopts the alias already bound to `uri` in scope,
    /// falling back to `default_alias` when the URI is not yet declared.
    ///
    /// Use this for the common case where a single canonical alias is acceptable and the
    /// document is free to reuse whatever alias an ancestor already declared for the URI.
    ///
    /// # Arguments
    /// * `uri` - The W3C namespace URI this declaration represents.
    /// * `default_alias` - The alias emitted when the URI is not already in scope.
    ///
    /// # Returns
    /// * `NamespaceDeclaration` - A declaration whose `alias_override` is `None`, so resolution
    ///   follows the precedence: in-scope alias for `uri` > `default_alias`.
    pub const fn new(uri: &'static str, default_alias: &'static str) -> Self {
        NamespaceDeclaration {
            uri,
            default_alias,
            alias_override: None,
        }
    }

    /// Builds a declaration that forces `alias_override` even when a different alias is
    /// already bound to `uri` in scope.
    ///
    /// Use this when a specific prefix must appear in the output regardless of what an
    /// ancestor declared; the resolver will (re)declare `alias_override -> uri` on the
    /// element being written when the binding is not already present.
    ///
    /// # Arguments
    /// * `uri` - The W3C namespace URI this declaration represents.
    /// * `default_alias` - Retained for parity with [`NamespaceDeclaration::new`]; unused while
    ///   `alias_override` is set but preserved so callers can drop the override later.
    /// * `alias_override` - The alias that resolution always selects for `uri`.
    ///
    /// # Returns
    /// * `NamespaceDeclaration` - A declaration whose resolution precedence is:
    ///   `alias_override` > in-scope alias for `uri` > `default_alias`.
    pub const fn with_override(
        uri: &'static str,
        default_alias: &'static str,
        alias_override: &'static str,
    ) -> Self {
        NamespaceDeclaration {
            uri,
            default_alias,
            alias_override: Some(alias_override),
        }
    }

    /// Resolves the alias for this declaration against `namespace`.
    ///
    /// Applies the precedence `alias_override` > alias already bound to `uri` > `default_alias`.
    ///
    /// # Arguments
    /// * `namespace` - The scope to resolve against.
    ///
    /// # Returns
    /// * `(alias, needs_declaration)` - The alias to use and whether the scope must
    ///   declare `alias -> uri` because it is not already bound to that URI.
    pub(crate) fn resolve_in(&self, namespace: &XmlNamespace) -> (String, bool) {
        if let Some(alias_override) = self.alias_override {
            let already_bound = namespace
                .get_url(alias_override)
                .map(|(bound_uri, _)| bound_uri == self.uri)
                .unwrap_or(false);
            return (alias_override.to_string(), !already_bound);
        }
        if let Some(existing_alias) = namespace.get_alias(self.uri) {
            return (existing_alias.clone(), false);
        }
        (self.default_alias.to_string(), true)
    }
}

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
    /// Aliases declared directly on this scope, in declaration order
    locally_declared_aliases: Vec<NsAlias>,
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
        if !self.locally_declared_aliases.iter().any(|declared_alias| declared_alias == alias) {
            self.locally_declared_aliases.push(alias.to_owned());
        }
    }

    /// Clears the record of aliases declared directly on this scope.
    pub(crate) fn clear_local_declarations_mut(&mut self) {
        self.locally_declared_aliases.clear();
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
    pub(crate) fn get_url(&self, alias: &str) -> Option<&(NsUrl, u32)> {
        self.alias_url.get(alias)
    }

    /// Gets the namespace alias for a given URL.
    ///
    /// # Arguments
    /// * `url` - The namespace URL.
    ///
    /// # Returns
    /// * `Option<&String>` - The alias/prefix if the URL is found, None otherwise.
    pub(crate) fn get_alias(&self, url: &str) -> Option<&String> {
        self.url_alias.get(url)
    }

    /// Returns the aliases declared directly on this scope with their URIs, in declaration order.
    pub(crate) fn get_local_declarations(&self) -> Vec<(NsAlias, NsUrl)> {
        self.locally_declared_aliases
            .iter()
            .filter_map(|alias| {
                self.alias_url
                    .get(alias)
                    .map(|(url, _)| (alias.clone(), url.clone()))
            })
            .collect()
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
            locally_declared_aliases: Vec::new(),
        }
    }
}
