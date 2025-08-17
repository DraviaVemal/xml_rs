/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{XmlAttribute, XmlElement, XmlElementContentType, XmlNamespace};
use anyhow::{Context, Error as AnyError};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

/// Type alias for node identifiers within an XML document.
pub type NodeId = u32;

/// Represents an XML document, containing elements in a tree structure.
///
/// This struct manages the entire XML document, including the root element,
/// version information, encoding, and a collection of all elements.
#[derive(Debug)]
pub struct XmlDocument {
    /// XML version string (Default, "1.0")
    version: String,
    /// XML document encoding (Default, "UTF-8")
    encoding: String,
    /// Counter for assigning unique IDs to nodes
    running_id: NodeId,
    /// Node ID of the root element
    root_id: NodeId,
    /// Collection of all elements in the document, indexed by their NodeId
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
    // --------------------------
    // pub mut self methods
    // --------------------------

    /// Creates and inserts the root element into the document.
    ///
    /// # Arguments
    /// * `tag` - The tag name for the root element.
    /// * `attributes` - Optional attributes for the root element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created root element, or an error.
    pub fn create_root_element_mut(
        &mut self,
        tag: String,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        // Generate a new unique ID for the element
        self.running_id += 1;
        let node_id = self.running_id;

        // Create the element with a new namespace context
        let mut element =
            XmlElement::new(tag, attributes, Rc::new(RefCell::new(XmlNamespace::new())))
                .context("Failed to create element")?;

        // Set the element's ID
        element.set_id_mut(node_id);

        // Add the element to the collection
        self.xml_element_collection.insert(node_id, element);

        Ok(node_id)
    }

    /// Adds a child element to a parent element.
    ///
    /// # Arguments
    /// * `parent_id` - The node ID of the parent element.
    /// * `tag` - The tag name for the child element.
    /// * `attributes` - Optional attributes for the child element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created child element, or an error.
    pub fn append_child_element_mut(
        &mut self,
        parent_id: NodeId,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        // Generate a new unique ID for the element
        self.running_id += 1;
        let node_id = self.running_id;

        // Get the namespace context from the parent element
        let ns_context = self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .get_ns_context();

        // Create the child element with the parent's namespace context
        let mut child_element = XmlElement::new(tag.to_string(), attributes, ns_context)
            .context("Failed to create child element")?;

        // Set the element's ID and parent ID
        child_element.set_id_mut(node_id);
        child_element.set_parent_id_mut(parent_id.clone());

        // Add the child element to the collection
        self.add_element(node_id, child_element);

        // Add the child to the parent's contents
        self.get_element_mut(parent_id)
            .context("Parent element not found")?
            .add_child_mut(node_id, tag.to_string())
            .context("Failed to add child element to parent")?;

        Ok(node_id)
    }

    /// Gets a mutable reference to an element by node ID.
    ///
    /// # Arguments
    /// * `active_xml_element_id` - The node ID to look up.
    ///
    /// # Returns
    /// * `Result<&mut XmlElement, AnyError>` - Mutable reference to the element or an error if not found.
    pub fn get_element_mut(
        &mut self,
        active_xml_element_id: NodeId,
    ) -> Result<&mut XmlElement, AnyError> {
        self.xml_element_collection
            .get_mut(&active_xml_element_id)
            .context("Get Element mut not found")
    }

    /// Clears the content of an element, removing all children.
    ///
    /// # Arguments
    /// * `element_id` - The node ID of the element to clear.
    ///
    /// # Returns
    /// * `Result<(), AnyError>` - Success or an error.
    pub fn clear_element_content_mut(&mut self, element_id: NodeId) -> Result<(), AnyError> {
        // Remove all child elements from the document
        self.clear_element_subtree_mut(element_id)?;

        // Clear the element's content
        self.get_element_mut(element_id)
            .context("Failed to get element")?
            .clear_content_mut();

        Ok(())
    }
}

impl XmlDocument {
    // --------------------------
    // pub self methods
    // --------------------------

    /// Gets the XML version string.
    ///
    /// # Returns
    /// * `&str` - The XML version (e.g., "1.0").
    pub fn get_version(&self) -> &str {
        &self.version
    }

    /// Gets the encoding string.
    ///
    /// # Returns
    /// * `&str` - The document encoding (e.g., "UTF-8").
    pub fn get_encoding(&self) -> &str {
        &self.encoding
    }

    /// Gets the root element's node ID.
    ///
    /// # Returns
    /// * `NodeId` - The ID of the root element.
    pub fn get_root_id(&self) -> NodeId {
        self.root_id
    }

    /// Gets a reference to an element by node ID.
    ///
    /// # Arguments
    /// * `active_xml_element_id` - The node ID to look up.
    ///
    /// # Returns
    /// * `Result<&XmlElement, AnyError>` - Reference to the element or an error if not found.
    pub fn get_element(&self, active_xml_element_id: NodeId) -> Result<&XmlElement, AnyError> {
        self.xml_element_collection
            .get(&active_xml_element_id)
            .context("Get Element not found")
    }

    /// Creates a clone of the document.
    ///
    /// # Returns
    /// * `XmlDocument` - A new document with the same content.
    pub fn clone(&self) -> XmlDocument {
        XmlDocument {
            version: self.version.clone(),
            encoding: self.encoding.clone(),
            running_id: self.running_id,
            root_id: self.root_id,
            // Clone each element in the collection
            xml_element_collection: self
                .xml_element_collection
                .iter()
                .map(|(node_id, element)| (node_id.clone(), element.clone_limited()))
                .collect(),
        }
    }

    /// Finds the first child element with the given tag name.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `tag` - The tag name to search for.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The ID of the first matching child, or None if not found.
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

    /// Finds the first child element with the given namespaced tag name.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `tag_ns` - The namespaced tag to search for (e.g., "ns:tag").
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The ID of the first matching child, or None if not found.
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

    /// Finds all child elements with the given tag name.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `tag` - The tag name to search for.
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - A vector of matching child IDs, or None if none found.
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

    /// Finds all child elements with the given namespaced tag name.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `tag_ns` - The namespaced tag to search for (e.g., "ns:tag").
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - A vector of matching child IDs, or None if none found.
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

    /// Finds the first child element with a specific attribute name and value.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `attr_name` - The attribute name to match.
    /// * `attr_value` - The attribute value to match.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The ID of the first matching child, or None if not found.
    pub fn find_first_by_attribute(
        &self,
        parent_id: NodeId,
        attr_name: &str,
        attr_value: &str,
    ) -> Result<Option<NodeId>, AnyError> {
        // Check if the parent element has contents
        if let Some(contents) = self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .get_contents()
        {
            // Iterate through each content item
            for content in contents {
                if let XmlElementContentType::Element((child_id, _)) = content {
                    // Check if the child element has the specified attribute with the specified value
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

    /// Finds the first child element with a specific namespaced attribute name and value.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `attr_name_ns` - The namespaced attribute name to match (e.g., "ns:attr").
    /// * `attr_value` - The attribute value to match.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The ID of the first matching child, or None if not found.
    pub fn find_first_by_attribute_ns(
        &self,
        parent_id: NodeId,
        attr_name_ns: &str,
        attr_value: &str,
    ) -> Result<Option<NodeId>, AnyError> {
        self.find_first_by_attribute(parent_id, attr_name_ns, attr_value)
    }

    /// Finds all child elements with a specific attribute name and value.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `attr_name` - The attribute name to match.
    /// * `attr_value` - The attribute value to match.
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - A vector of matching child IDs, or None if none found.
    pub fn find_all_by_attribute(
        &self,
        parent_id: NodeId,
        attr_name: &str,
        attr_value: &str,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        let mut result = Vec::new();

        // Check if the parent element has contents
        if let Some(contents) = self
            .get_element(parent_id)
            .context("Failed to pull parent element")?
            .get_contents()
        {
            // Iterate through each content item
            for content in contents {
                if let XmlElementContentType::Element((child_id, _)) = content {
                    // Check if the child element has the specified attribute with the specified value
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

        // Return None if no matching children found
        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    /// Finds all child elements with a specific namespaced attribute name and value.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `attr_name_ns` - The namespaced attribute name to match (e.g., "ns:attr").
    /// * `attr_value` - The attribute value to match.
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - A vector of matching child IDs, or None if none found.
    pub fn find_all_by_attribute_ns(
        &self,
        parent_id: NodeId,
        attr_name_ns: &str,
        attr_value: &str,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        self.find_all_by_attribute(parent_id, attr_name_ns, attr_value)
    }

    /// Removes an element and all its descendants from the document.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the element to remove.
    ///
    /// # Returns
    /// * `Result<(), AnyError>` - Success or an error.
    pub fn remove_element_mut(&mut self, element_id: NodeId) -> Result<(), AnyError> {
        // Get the parent ID of the element
        if let Some(parent_id) = self
            .get_element_mut(element_id)
            .context("Failed to get element")?
            .get_parent_id()
        {
            // Remove the element from its parent's contents
            if let Some(parent) = self
                .get_element_mut(parent_id)
                .context("Failed to get parent element")?
                .get_contents_mut()
            {
                // Filter out the element from parent's contents
                parent.retain(|content| match content {
                    XmlElementContentType::Element((child_id, _)) => *child_id != element_id,
                    _ => true,
                });
            }
        }

        // Remove all descendant elements recursively
        self.clear_element_subtree_mut(element_id)
            .context("Failed to clean up child element tree")?;

        // Remove the element itself from the collection
        self.xml_element_collection.remove(&element_id);

        Ok(())
    }
}

impl XmlDocument {
    // --------------------------
    // pub constructor
    // --------------------------

    /// Creates a new XML document with default version and encoding.
    ///
    /// # Returns
    /// * `XmlDocument` - A new XML document instance.
    pub fn new() -> XmlDocument {
        XmlDocument::default()
    }
}

impl XmlDocument {
    // --------------------------
    // pub(crate) methods
    // --------------------------

    /// Adds an element to the document collection.
    ///
    /// # Arguments
    /// * `id` - The node ID for the element.
    /// * `element` - The element to add.
    pub(crate) fn add_element(&mut self, id: NodeId, element: XmlElement) {
        self.xml_element_collection.insert(id, element);
    }

    /// Sets the XML version string.
    ///
    /// # Arguments
    /// * `version` - The version string to set (e.g., "1.0").
    pub(crate) fn set_version_mut(&mut self, version: String) {
        self.version = version;
    }

    /// Sets the XML document encoding.
    ///
    /// # Arguments
    /// * `encoding` - The encoding string to set (e.g., "UTF-8").
    pub(crate) fn set_encoding_mut(&mut self, encoding: String) {
        self.encoding = encoding
    }

    /// Recursively removes all descendant elements present in the contents of the given element.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the element whose entire subtree should be cleared.
    ///
    /// # Returns
    /// * `Result<(), AnyError>` - Success or an error.
    pub(crate) fn clear_element_subtree_mut(&mut self, element_id: NodeId) -> Result<(), AnyError> {
        // Make a copy of the contents to avoid borrowing issues during iteration
        if let Some(contents) = self
            .get_element_mut(element_id)
            .context("Failed to get element")?
            .get_contents()
            .clone()
        {
            // Process each content item
            for content in contents {
                if let XmlElementContentType::Element((child_id, _)) = content {
                    // Recursively clear the subtree of each child element
                    self.clear_element_subtree_mut(child_id)?;
                }
            }
        }
        Ok(())
    }
}
