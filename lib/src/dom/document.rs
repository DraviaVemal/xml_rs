// Copyright (c) DraviaVemal 2025
// Licensed under the Sponsorware License v4.0+ (see LICENSE for details).

use anyhow::{Context, Error as AnyError};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::{XmlAttribute, XmlElement, XmlNamespace};

pub type NodeId = u32;

#[derive(Debug, Clone)]
pub struct XmlDocument {
    version: String,
    encoding: String,
    running_id: NodeId,
    root_id: NodeId,
    xml_element_collection: BTreeMap<NodeId, XmlElement>,
}

impl Default for XmlDocument {
    fn default() -> Self {
        XmlDocument {
            version: "1.0".into(),
            encoding: "UTF-8".into(),
            running_id: 0,
            root_id: 1,
            xml_element_collection: BTreeMap::new(),
        }
    }
}

impl XmlDocument {
    /// Create a new XML document with default version and encoding.
    ///
    /// # Returns
    /// * `XmlDocument`
    pub fn new() -> XmlDocument {
        XmlDocument::default()
    }

    /// Get the XML version string.
    ///
    /// # Returns
    /// * `&str`
    pub fn get_version(&self) -> &str {
        &self.version
    }

    /// Get the encoding string.
    ///
    /// # Returns
    /// * `&str`
    pub fn get_encoding(&self) -> &str {
        &self.encoding
    }

    /// Get the root element's node ID.
    ///
    /// # Returns
    /// * `&NodeId`
    pub fn get_root_id(&self) -> &NodeId {
        &self.root_id
    }

    /// Get a reference to an element by node ID.
    ///
    /// # Arguments
    /// * `active_xml_element_id` - The node ID to look up.
    ///
    /// # Returns
    /// * `Result<&XmlElement, AnyError>`
    pub fn get_element(&self, active_xml_element_id: &NodeId) -> Result<&XmlElement, AnyError> {
        self.xml_element_collection
            .get(&active_xml_element_id)
            .context("Element not found")
    }
}

impl XmlDocument {
    /// Create and insert the root element.
    ///
    /// # Arguments
    /// * `tag` - The tag name for the root.
    /// * `attributes` - Optional attributes for the root.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>`
    pub fn create_root_element_mut(
        &mut self,
        tag: String,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        self.running_id += 1;
        let node_id = self.running_id;
        let mut element =
            XmlElement::new(tag, attributes, Rc::new(RefCell::new(XmlNamespace::new())))
                .context("Failed to create element")?;
        element.set_id_mut(node_id);
        self.xml_element_collection.insert(node_id, element);
        Ok(node_id)
    }

    /// Add a child element to a parent element.
    ///
    /// # Arguments
    /// * `parent_id` - The parent node ID.
    /// * `tag` - The tag name for the child.
    /// * `attributes` - Optional attributes for the child.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>`
    pub fn add_child_element_mut(
        &mut self,
        parent_id: &NodeId,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        self.running_id += 1;
        let node_id = self.running_id;
        let ns_context = self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .get_ns_context();
        let mut child_element = XmlElement::new(tag.to_string(), attributes, ns_context)
            .context("Failed to create child element")?;
        child_element.set_id_mut(node_id);
        child_element.set_parent_id_mut(parent_id.clone());
        self.add_element(node_id, child_element);
        self.get_element_mut(&parent_id)
            .context("Parent element not found")?
            .add_child_mut(tag.to_string(), node_id);
        Ok(node_id)
    }

    pub(crate) fn set_version_mut(&mut self, version: String) {
        self.version = version;
    }

    pub(crate) fn set_encoding_mut(&mut self, encoding: String) {
        self.encoding = encoding
    }
}

// Do all xml element collection in methods so its organised
impl XmlDocument {
    pub(crate) fn add_element(&mut self, id: NodeId, element: XmlElement) {
        self.xml_element_collection.insert(id, element);
    }

    pub(crate) fn get_element_mut(
        &mut self,
        active_xml_element_id: &NodeId,
    ) -> Result<&mut XmlElement, AnyError> {
        self.xml_element_collection
            .get_mut(&active_xml_element_id)
            .context("Element not found")
    }
}
