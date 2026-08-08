/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::XmlNamespace;

/// Represents an XML attribute with optional namespace information.
///
/// This struct stores the name, value, and optional namespace alias for an XML attribute.
/// It handles both regular attributes and namespaced attributes (e.g., "ns:attr").
#[derive(Debug, Clone, Default)]
pub struct XmlAttribute {
    /// Local name of the attribute (without namespace prefix)
    name: String,
    /// Value of the attribute
    value: String,
    /// Namespace alias/prefix for this attribute, if any
    ns_alias: Option<String>,
}

impl XmlAttribute {
    // =====================================================================
    //  DEVELOPER HACK — direct construction from a raw name (maximum flexibility)
    // =====================================================================

    /// Constructs a new `XmlAttribute` from a raw name and value.
    ///
    /// The `name` is split on the first `':'` into an optional namespace alias and a local
    /// name, so `"r:embed"` yields alias `"r"` and local name `"embed"`. The alias is stored
    /// verbatim and is **not** validated against any namespace scope here; validation happens
    /// when the attribute is attached to an element.
    ///
    /// # Recommendation
    /// This is the low-level primitive and gives full control over the emitted prefix. For a
    /// robust document prefer [`crate::XmlElement::add_attribute_ns_mut`], which resolves the
    /// alias from the live scope (reusing an existing binding or declaring a new one) instead
    /// of trusting a hard-coded prefix string.
    ///
    /// # Arguments
    /// * `name` - The attribute name, optionally namespaced (e.g., "ns:attr").
    /// * `value` - The attribute value.
    ///
    /// # Returns
    /// * `XmlAttribute` - The constructed attribute with parsed namespace information.
    pub fn new(name: String, value: String) -> XmlAttribute {
        // Split name into namespace alias and local name if ':' is present
        let (ns_alias, name) = if let Some(pos) = name.find(':') {
            // Extract the namespace prefix and the local name
            let (ns, tag) = name.split_at(pos);
            (Some(ns.to_string()), tag[1..].to_string())
        } else {
            // No namespace prefix
            (None, name)
        };

        XmlAttribute {
            name,
            value,
            ns_alias,
        }
    }
}

impl XmlAttribute {
    // =====================================================================
    //  SHARED — neutral accessors
    // =====================================================================

    /// Returns the attribute's local name without namespace prefix.
    ///
    /// # Returns
    /// * `&str` - The local name of the attribute (e.g., "embed" for "r:embed").
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the attribute's name with its namespace alias if present.
    ///
    /// The alias returned is the one stored on the attribute, not a scope-resolved prefix.
    ///
    /// # Returns
    /// * `String` - The namespaced name (e.g., "ns:attr") or just the local name if no alias.
    pub fn get_ns_name(&self) -> String {
        match &self.ns_alias {
            Some(alias) if !alias.is_empty() => format!("{}:{}", alias, self.name),
            _ => self.name.clone(),
        }
    }

    /// Returns the value of the attribute.
    ///
    /// # Returns
    /// * `&str` - The value of the attribute.
    pub fn get_value(&self) -> &str {
        &self.value
    }

    /// Returns the namespace alias/prefix stored on the attribute, if any.
    ///
    /// # Returns
    /// * `Option<&str>` - The alias (e.g., "r") or `None` when the attribute has no prefix.
    pub fn get_ns_alias(&self) -> Option<&str> {
        self.ns_alias.as_deref()
    }

    /// Validates that this attribute's alias (if any) is declared in `namespace_context`.
    ///
    /// # Returns
    /// * `bool` - `true` when the attribute is unprefixed or its alias is declared in scope.
    pub(crate) fn is_valid_ns_alias(&self, namespace_context: &XmlNamespace) -> bool {
        match self.ns_alias.as_ref() {
            Some(ns) => namespace_context.is_valid_ns_alias(ns),
            None => true,
        }
    }
}
