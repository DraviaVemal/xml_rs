/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{XmlAttribute, XmlElement, XmlElementContentType, XmlNamespace};
use anyhow::{Context, Error as AnyError};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

pub type NodeId = u32;

#[derive(Debug)]
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
    pub fn append_child_element_mut(
        &mut self,
        parent_id: NodeId,
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
        self.get_element_mut(parent_id)
            .context("Parent element not found")?
            .add_child_mut(node_id, tag.to_string());
        Ok(node_id)
    }

    pub fn clear_element_content_mut(&mut self, element_id: NodeId) -> Result<(), AnyError> {
        self.clear_element_subtree_mut(element_id)?;
        self.get_element_mut(element_id)
            .context("Failed to get element")?
            .clear_content_mut();
        Ok(())
    }
}

impl XmlDocument {
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
    /// * `NodeId`
    pub fn get_root_id(&self) -> NodeId {
        self.root_id
    }

    /// Get a reference to an element by node ID.
    ///
    /// # Arguments
    /// * `active_xml_element_id` - The node ID to look up.
    ///
    /// # Returns
    /// * `Result<&XmlElement, AnyError>`
    pub fn get_element(&self, active_xml_element_id: NodeId) -> Result<&XmlElement, AnyError> {
        self.xml_element_collection
            .get(&active_xml_element_id)
            .context("Element not found")
    }

    pub fn clone(&self) -> XmlDocument {
        XmlDocument {
            version: self.version.clone(),
            encoding: self.encoding.clone(),
            running_id: self.running_id,
            root_id: self.root_id,
            xml_element_collection: self
                .xml_element_collection
                .iter()
                .map(|(node_id, element)| (node_id.clone(), element.clone_limited()))
                .collect(),
        }
    }

    pub fn find_first_child(
        &self,
        parent_id: NodeId,
        tag: &str,
    ) -> Result<Option<NodeId>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .find_first_child(tag))
    }

    pub fn find_first_child_ns(
        &self,
        parent_id: NodeId,
        tag_ns: &str,
    ) -> Result<Option<NodeId>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .find_first_child_ns(tag_ns))
    }

    pub fn find_all_child(
        &self,
        parent_id: NodeId,
        tag: &str,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .find_all_child(tag))
    }

    pub fn find_all_child_ns(
        &self,
        parent_id: NodeId,
        tag_ns: &str,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .find_all_child_ns(tag_ns))
    }

    pub fn find_first_by_attribute(
        &self,
        parent_id: NodeId,
        attr_name: &str,
        attr_value: &str,
    ) -> Result<Option<NodeId>, AnyError> {
        if let Some(contents) = self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .get_contents()
        {
            for content in contents {
                if let XmlElementContentType::Element((child_id, _)) = content {
                    if self
                        .get_element(*child_id)
                        .context("Failed to pull child element")?
                        .has_attribute(attr_name, attr_value)
                    {
                        return Ok(Some(child_id.clone()));
                    }
                }
            }
        }
        Ok(None)
    }

    pub fn find_first_by_attribute_ns(
        &self,
        parent_id: NodeId,
        attr_name_ns: &str,
        attr_value: &str,
    ) -> Result<Option<NodeId>, AnyError> {
        self.find_first_by_attribute(parent_id, attr_name_ns, attr_value)
    }

    pub fn find_all_by_attribute(
        &self,
        parent_id: NodeId,
        attr_name: &str,
        attr_value: &str,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        let mut result = Vec::new();
        if let Some(contents) = self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .get_contents()
        {
            for content in contents {
                if let XmlElementContentType::Element((child_id, _)) = content {
                    if self
                        .get_element(*child_id)
                        .context("Failed to pull child element")?
                        .has_attribute(attr_name, attr_value)
                    {
                        result.push(child_id.clone());
                    }
                }
            }
        }
        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    pub fn find_all_by_attribute_ns(
        &self,
        parent_id: NodeId,
        attr_name_ns: &str,
        attr_value: &str,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        self.find_all_by_attribute(parent_id, attr_name_ns, attr_value)
    }

    /// Will Remove the target element and all its child tree items
    /// # Arguments
    /// - `element_id` (`NodeId`) - The ID of the element to remove.
    /// # Returns
    /// - `Result<(), AnyError>` - Indicates success or failure.
    pub fn remove_element_mut(&mut self, element_id: NodeId) -> Result<(), AnyError> {
        if let Some(parent_id) = self
            .get_element_mut(element_id)
            .context("Failed to get element")?
            .get_parent_id()
        {
            if let Some(parent) = self
                .get_element_mut(parent_id)
                .context("Failed to get parent element")?
                .get_contents_mut()
            {
                parent.retain(|content| match content {
                    XmlElementContentType::Element((child_id, _)) => *child_id != element_id,
                    _ => true,
                });
            }
        }
        self.clear_element_subtree_mut(element_id)
            .context("Failed to clean up child element tree")?;
        self.xml_element_collection.remove(&element_id);
        Ok(())
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
}

impl XmlDocument {
    pub(crate) fn add_element(&mut self, id: NodeId, element: XmlElement) {
        self.xml_element_collection.insert(id, element);
    }

    pub(crate) fn get_element_mut(
        &mut self,
        active_xml_element_id: NodeId,
    ) -> Result<&mut XmlElement, AnyError> {
        self.xml_element_collection
            .get_mut(&active_xml_element_id)
            .context("Element not found")
    }

    pub(crate) fn set_version_mut(&mut self, version: String) {
        self.version = version;
    }

    pub(crate) fn set_encoding_mut(&mut self, encoding: String) {
        self.encoding = encoding
    }
    /// Recursively removes all descendant elements present in the contents of the given element.
    ///
    /// # Arguments
    /// - `element_id` (`NodeId`) - The ID of the element whose entire subtree should be cleared.
    /// # Returns
    /// - `Result<(), AnyError>` - Indicates success or failure.
    pub(crate) fn clear_element_subtree_mut(&mut self, element_id: NodeId) -> Result<(), AnyError> {
        if let Some(contents) = self
            .get_element_mut(element_id)
            .context("Failed to get element")?
            .get_contents()
            .clone()
        {
            for content in contents {
                match content {
                    XmlElementContentType::Element((child_id, _)) => {
                        self.clear_element_subtree_mut(child_id)?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
