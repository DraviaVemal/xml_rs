/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

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
    // --------------------------
    // pub self methods
    // --------------------------

    /// Returns the attribute's local name without namespace prefix.
    ///
    /// # Returns
    /// * `&str` - The local name of the attribute.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the attribute's name with namespace alias if present.
    ///
    /// # Returns
    /// * `String` - The namespaced name (e.g., "ns:attr") or just the local name if no namespace.
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
}

impl XmlAttribute {
    // --------------------------
    // pub constructor
    // --------------------------
    
    /// Constructs a new `XmlAttribute` from a name and value.
    ///
    /// # Arguments
    /// * `name` - The attribute name, possibly namespaced (e.g., "ns:attr").
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
