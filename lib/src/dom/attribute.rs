// Copyright (c) DraviaVemal 2025
// Licensed under the Sponsorware License v4.0+ (see LICENSE for details).

#[derive(Debug, Clone, Default)]
pub struct XmlAttribute {
    name: String,
    value: String,
    _ns_link: Option<String>,
    ns_alias: Option<String>,
}

impl XmlAttribute {
    /// Returns the attribute's local name.
    ///
    /// # Returns
    /// * `&str` - The name of the attribute.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the attribute's name with namespace alias if present.
    ///
    /// # Returns
    /// * `String` - The namespaced name (e.g., "ns:attr") or just the name.
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
    /// Constructs a new `XmlAttribute` from a name and value.
    ///
    /// # Arguments
    /// * `name` - The attribute name, possibly namespaced (e.g., "ns:attr").
    /// * `value` - The attribute value.
    ///
    /// # Returns
    /// * `XmlAttribute` - The constructed attribute.
    pub fn new(name: String, value: String) -> XmlAttribute {
        // Split name into namespace alias and local name if ':' is present
        let (ns_alias, name) = if let Some(pos) = name.find(':') {
            let (ns, tag) = name.split_at(pos);
            (Some(ns.to_string()), tag[1..].to_string())
        } else {
            (None, name)
        };
        XmlAttribute {
            name,
            value,
            _ns_link: None,
            ns_alias,
        }
    }
}
