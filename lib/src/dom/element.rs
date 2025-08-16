// Copyright (c) DraviaVemal 2025
// Licensed under the Sponsorware License v4.0+ (see LICENSE for details).

use std::{cell::RefCell, rc::Rc};

use crate::{NodeId, XmlAttribute, XmlNamespace, utils::validation};
use anyhow::Error as AnyError;

#[derive(Debug, Clone)]
pub enum XmlElementContentType {
    Element((NodeId, String)),
    Text(String),
    Comment(String),
}

/// XML element node.
#[derive(Debug, Clone)]
pub struct XmlElement {
    id: NodeId,
    tag: String,
    contents: Option<Vec<XmlElementContentType>>,
    parent_id: Option<NodeId>,
    ns_alias: Option<String>,
    attributes: Option<Vec<XmlAttribute>>,
    ns_context_override: bool,
    namespace_context: Rc<RefCell<XmlNamespace>>,
}

impl XmlElement {
    /// Get the unique node ID of this element.
    ///
    /// # Returns
    /// * `NodeId` - The element's ID.
    pub fn get_id(&self) -> NodeId {
        self.id
    }

    /// Get the parent node ID, if any.
    ///
    /// # Returns
    /// * `Option<NodeId>` - The parent element's ID.
    pub fn get_parent_id(&self) -> Option<NodeId> {
        self.parent_id
    }

    /// Get the tag name of this element (without namespace).
    ///
    /// # Returns
    /// * `String` - The tag name.
    pub fn get_tag(&self) -> String {
        self.tag.clone()
    }

    /// Get the tag name with namespace alias if present.
    ///
    /// # Returns
    /// * `String` - The namespaced tag name.
    pub fn get_tag_ns(&self) -> String {
        match &self.ns_alias {
            Some(ns_alias) if !ns_alias.is_empty() => format!("{}:{}", ns_alias, self.tag),
            _ => self.tag.clone(),
        }
    }

    /// Get a reference to the element's attributes.
    ///
    /// # Returns
    /// * `&Option<Vec<XmlAttribute>>` - The attributes, if any.
    pub fn get_attributes(&self) -> &Option<Vec<XmlAttribute>> {
        &self.attributes
    }

    /// Get a reference to the element's contents (children, text, comments).
    ///
    /// # Returns
    /// * `&Option<Vec<XmlElementContentType>>` - The contents, if any.
    pub fn get_contents(&self) -> &Option<Vec<XmlElementContentType>> {
        &self.contents
    }

    pub(crate) fn get_ns_context(&self) -> Rc<RefCell<XmlNamespace>> {
        self.namespace_context.clone()
    }
}

impl XmlElement {
    /// Add a child element by tag and node ID.
    ///
    /// # Arguments
    /// * `tag` - The tag name of the child.
    /// * `child_id` - The node ID of the child.
    pub fn add_child_mut(&mut self, tag: String, child_id: NodeId) {
        // Ensure contents vector exists
        if self.contents.is_none() {
            self.contents = Some(Vec::new());
        }
        self.contents
            .as_mut()
            .unwrap()
            .push(XmlElementContentType::Element((child_id, tag)));
    }

    /// Add an attribute to this element.
    ///
    /// # Arguments
    /// * `attribute` - The attribute to add.
    pub fn add_attribute_mut(&mut self, attribute: XmlAttribute) {
        // Ensure attributes vector exists
        if self.attributes.is_none() {
            self.attributes = Some(Vec::new());
        }
        self.attributes.as_mut().unwrap().push(attribute);
    }

    /// Remove an attribute by its local name.
    ///
    /// # Arguments
    /// * `name` - The name of the attribute to remove.
    pub fn remove_attribute_mut(&mut self, name: &str) {
        if let Some(attributes) = &mut self.attributes {
            attributes.retain(|a| a.get_name() != name);
        }
    }

    /// Remove an attribute by its namespaced name.
    ///
    /// # Arguments
    /// * `ns_name` - The namespaced name to remove.
    pub fn remove_attribute_ns_mut(&mut self, ns_name: &str) {
        if let Some(attributes) = &mut self.attributes {
            attributes.retain(|a| !(a.get_ns_name() == ns_name));
        }
    }

    /// Add content (child, text, or comment) to this element.
    ///
    /// # Arguments
    /// * `content_type` - The content to add.
    ///
    /// # Returns
    /// * `&mut Self` - For chaining.
    pub fn add_content_mut(&mut self, content_type: XmlElementContentType) -> &mut Self {
        if self.contents.is_none() {
            self.contents = Some(Vec::new());
        }
        self.contents.as_mut().unwrap().push(content_type);
        self
    }

    /// Get a mutable reference to the contents.
    ///
    /// # Returns
    /// * `&mut Option<Vec<XmlElementContentType>>`
    pub fn get_contents_mut(&mut self) -> &mut Option<Vec<XmlElementContentType>> {
        &mut self.contents
    }

    pub(crate) fn set_id_mut(&mut self, id: NodeId) {
        self.id = id;
    }

    pub(crate) fn set_parent_id_mut(&mut self, parent_id: NodeId) {
        self.parent_id = Some(parent_id);
    }
}

impl XmlElement {
    pub(crate) fn new(
        tag: String,
        attributes: Option<Vec<XmlAttribute>>,
        mut namespace_context: Rc<RefCell<XmlNamespace>>,
    ) -> Result<XmlElement, AnyError> {
        let mut ns_context_override = false;
        if validation::is_valid_xml_name(&tag) {
            if let Some(attributes) = attributes.as_ref() {
                if !attributes
                    .iter()
                    .all(|attribute| validation::is_valid_xml_name(&attribute.get_ns_name()))
                {
                    return Err(AnyError::msg("Not all attributes satisfy naming standards"));
                }
                let mut attributes = attributes.clone();
                let mut namespaces = Vec::new();
                attributes.retain(|attribute| {
                    if attribute.get_ns_name().starts_with("xmlns") {
                        namespaces.push(attribute.clone());
                        false
                    } else {
                        true
                    }
                });
                if !namespaces.is_empty() {
                    ns_context_override = true;
                    namespace_context = Rc::new(RefCell::new(XmlNamespace::new()));
                    for namespace in namespaces {
                        namespace_context.borrow_mut().add_namespace(namespace);
                    }
                }
            }
            let (ns_alias, tag) = if let Some(pos) = tag.find(':') {
                let (ns, tag) = tag.split_at(pos);
                (Some(ns.to_string()), tag[1..].to_string())
            } else {
                (None, tag)
            };
            Ok(XmlElement {
                id: 0,
                tag,
                attributes,
                parent_id: None,
                contents: None,
                ns_alias,
                ns_context_override,
                namespace_context,
            })
        } else {
            Err(AnyError::msg("Invalid XML tag name"))
        }
    }
}
