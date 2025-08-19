/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{utils::validation, NodeId, XmlAttribute, XmlNamespace};
use anyhow::{Context, Error as AnyError};
use std::{cell::RefCell, rc::Rc};

pub type Tag = String;
pub type NsTag = String;

/// Represents the different types of content that can be contained within an XML element.
///
/// This enum differentiates between child elements, text nodes, and comment nodes.
#[derive(Debug, Clone)]
pub enum XmlElementContentType {
    /// A child element represented by its NodeId and tag name.
    Element((NodeId, Tag, NsTag)),
    /// A text node containing plain text content.
    Text(String),
    /// A comment node containing comment text.
    Comment(String),
}

/// Represents an XML element node in the DOM tree.
///
/// This struct contains all the information related to an XML element including
/// its tag name, attributes, content, namespace information, and position in the tree.
#[derive(Debug)]
pub struct XmlElement {
    /// Unique identifier for this element within the document
    id: NodeId,
    /// Local name of the element (without namespace prefix)
    tag: Tag,
    /// Child elements, text nodes, and comments contained in this element
    child_contents: Option<Vec<XmlElementContentType>>,
    /// Reference to the parent element's ID, if any
    parent_id: Option<NodeId>,
    /// Namespace alias/prefix for this element, if any
    ns_alias: Option<String>,
    /// Attributes attached to this element
    attributes: Option<Vec<XmlAttribute>>,
    /// Whether this element has its own namespace context that overrides parent's
    ns_context_override: bool,
    /// Reference to the namespace context for resolving prefixes
    namespace_context: Rc<RefCell<XmlNamespace>>,
}

impl XmlElement {
    // --------------------------
    // pub mut self methods
    // --------------------------

    /// Adds an attribute to this element.
    ///
    /// # Arguments
    /// * `attribute` - The XML attribute to add to this element.
    pub fn add_attribute_mut(&mut self, attribute: XmlAttribute) -> Result<(), AnyError> {
        // Ensure attributes vector exists before adding a new attribute
        if self.attributes.is_none() {
            self.attributes = Some(Vec::new());
        }
        // Add the attribute to the attributes collection
        self.attributes
            .as_mut()
            .context("Failed to insert value")?
            .push(attribute);
        Ok(())
    }

    /// Clear all attribute of this element.
    pub fn clear_attribute_mut(&mut self) -> Result<(), AnyError> {
        self.attributes = None;
        Ok(())
    }

    /// Removes an attribute by its local name.
    ///
    /// # Arguments
    /// * `name` - The local name of the attribute to remove.
    pub fn remove_attribute_mut(&mut self, name: &str) {
        if let Some(attributes) = &mut self.attributes {
            // Filter out the attribute with the matching name
            attributes.retain(|a| a.get_name() != name);
        }
    }

    /// Removes an attribute by its namespaced name.
    ///
    /// # Arguments
    /// * `ns_name` - The namespaced name of the attribute to remove (e.g., "ns:attr").
    pub fn remove_attribute_ns_mut(&mut self, ns_name: &str) {
        if let Some(attributes) = &mut self.attributes {
            // Filter out the attribute with the matching namespaced name
            attributes.retain(|a| !(a.get_ns_name() == ns_name));
        }
    }

    /// Gets a mutable reference to the contents collection.
    ///
    /// # Returns
    /// * `&mut Option<Vec<XmlElementContentType>>` - Mutable reference to the optional contents.
    pub fn get_child_contents_mut(&mut self) -> &mut Option<Vec<XmlElementContentType>> {
        &mut self.child_contents
    }

    /// Adds a text node to this element's contents.
    ///
    /// # Arguments
    /// * `text` - The text content to add.
    pub fn add_text_mut(&mut self, text: &str) -> Result<(), AnyError> {
        self.add_child_content_mut(XmlElementContentType::Text(text.to_owned()))?;
        Ok(())
    }

    /// Adds a comment node to this element's contents.
    ///
    /// This method allows you to insert XML comments (`<!-- comment -->`) into the element.
    /// Comments are preserved during serialization and can be used for documentation
    /// or to temporarily disable parts of the XML.
    ///
    /// # Arguments
    /// * `comment` - The comment text to add (without the `<!--` and `-->` delimiters).
    ///
    /// # Returns
    /// * `Result<&mut XmlElement, AnyError>` - A mutable reference to self for method chaining,
    ///   or an error if adding the comment failed.
    ///
    /// # Example
    /// ```
    /// element.add_comments_mut("This section contains user information")?;
    /// ```
    pub fn add_comments_mut(&mut self, comment: &str) -> Result<(), AnyError> {
        self.add_child_content_mut(XmlElementContentType::Comment(comment.to_owned()))?;
        Ok(())
    }
}

impl XmlElement {
    // --------------------------
    // pub self methods
    // --------------------------

    /// Gets the unique node ID of this element.
    ///
    /// # Returns
    /// * `NodeId` - The element's unique identifier within the document.
    pub fn get_id(&self) -> NodeId {
        self.id
    }

    /// Gets the parent node ID, if any.
    ///
    /// # Returns
    /// * `Option<NodeId>` - The parent element's ID, or None if this is the root element.
    pub fn get_parent_id(&self) -> Option<NodeId> {
        self.parent_id
    }

    /// Gets the tag name of this element (without namespace).
    ///
    /// # Returns
    /// * `String` - The local tag name without any namespace prefix.
    pub fn get_tag(&self) -> String {
        self.tag.clone()
    }

    /// Gets the tag name with namespace alias if present.
    ///
    /// # Returns
    /// * `String` - The namespaced tag name (e.g., "ns:tag") or just the tag if no namespace.
    pub fn get_tag_ns(&self) -> String {
        match &self.ns_alias {
            Some(ns_alias) if !ns_alias.is_empty() => format!("{}:{}", ns_alias, self.tag),
            _ => self.tag.clone(),
        }
    }

    /// Retrieves an attribute by its local name.
    ///
    /// # Arguments
    /// * `name` - The local name of the attribute to retrieve.
    ///
    /// # Returns
    /// * `Option<&XmlAttribute>` - A reference to the attribute if found, or None.
    pub fn get_attribute(&self, name: &str) -> Option<&XmlAttribute> {
        if let Some(attributes) = self.attributes.as_ref() {
            attributes.iter().find(|item| item.get_name() == name)
        } else {
            None
        }
    }

    /// Retrieves an attribute by its namespaced name.
    ///
    /// # Arguments
    /// * `name_ns` - The namespaced name of the attribute to retrieve (e.g., "ns:attr").
    ///
    /// # Returns
    /// * `Option<&XmlAttribute>` - A reference to the attribute if found, or None.
    pub fn get_attribute_ns(&self, name_ns: &str) -> Option<&XmlAttribute> {
        if let Some(attributes) = self.attributes.as_ref() {
            attributes.iter().find(|item| item.get_ns_name() == name_ns)
        } else {
            None
        }
    }

    /// Gets a reference to the element's contents (children, text, comments).
    ///
    /// # Returns
    /// * `&Option<Vec<XmlElementContentType>>` - The contents, if any.
    pub fn get_child_contents(&self) -> &Option<Vec<XmlElementContentType>> {
        &self.child_contents
    }

    /// Gets the count of child elements.
    ///
    /// # Returns
    /// * `Result<u32, AnyError>` - The count of child elements, or an error if the contents are not accessible.
    pub fn get_child_element_count(&self) -> Result<u32, AnyError> {
        let count = self
            .child_contents
            .as_ref()
            .context("Failed to open contents")?
            .iter()
            .filter(|content| match content {
                XmlElementContentType::Element(_) => true,
                _ => false,
            })
            .count() as u32;
        Ok(count)
    }

    /// Finds the first child element with the given tag name.
    ///
    /// # Arguments
    /// * `tag` - The tag name to search for.
    ///
    /// # Returns
    /// * `Option<NodeId>` - The NodeId of the first matching child, or None if not found.
    pub fn find_first_child(&self, tag: &str) -> Option<NodeId> {
        // Check if contents exist, then find the first child element with matching tag
        self.child_contents
            .as_ref()?
            .iter()
            .find_map(|content| match content {
                XmlElementContentType::Element((child_id, child_tag, _)) if child_tag == tag => {
                    Some(*child_id)
                }
                _ => None,
            })
    }

    /// Finds the first child element with the given tag name.
    ///
    /// # Arguments
    /// * `tag_ns` - The tag name to search for with nsmaespace.
    ///
    /// # Returns
    /// * `Option<NodeId>` - The NodeId of the first matching child, or None if not found.
    pub fn find_first_child_ns(&self, tag_ns: &str) -> Option<NodeId> {
        // Check if contents exist, then find the first child element with matching tag
        self.child_contents
            .as_ref()?
            .iter()
            .find_map(|content| match content {
                XmlElementContentType::Element((child_id, _, child_tag_ns))
                    if child_tag_ns == tag_ns =>
                {
                    Some(*child_id)
                }
                _ => None,
            })
    }

    /// Finds all child elements with the given tag name.
    ///
    /// # Arguments
    /// * `tag` - The tag name to search for.
    ///
    /// # Returns
    /// * `Option<Vec<NodeId>>` - A vector of matching child NodeIds, or None if none found.
    pub fn find_all_child(&self, tag: &str) -> Option<Vec<NodeId>> {
        // Collect all child elements with matching tag into a vector
        let childs: Vec<NodeId> = self
            .child_contents
            .as_ref()?
            .iter()
            .filter_map(|content| match content {
                XmlElementContentType::Element((child_id, child_tag, _)) if child_tag == tag => {
                    Some(*child_id)
                }
                _ => None,
            })
            .collect();

        // Return None if no matching children found
        if childs.is_empty() {
            None
        } else {
            Some(childs)
        }
    }

    /// Finds all child elements with the given tag name.
    ///
    /// # Arguments
    /// * `tag_ns` - The tag name to search for.
    ///
    /// # Returns
    /// * `Option<Vec<NodeId>>` - A vector of matching child NodeIds, or None if none found.
    pub fn find_all_child_ns(&self, tag_ns: &str) -> Option<Vec<NodeId>> {
        // Collect all child elements with matching tag into a vector
        let childs: Vec<NodeId> = self
            .child_contents
            .as_ref()?
            .iter()
            .filter_map(|content| match content {
                XmlElementContentType::Element((child_id, _, child_tag_ns))
                    if child_tag_ns == tag_ns =>
                {
                    Some(*child_id)
                }
                _ => None,
            })
            .collect();

        // Return None if no matching children found
        if childs.is_empty() {
            None
        } else {
            Some(childs)
        }
    }
}

impl XmlElement {
    // --------------------------
    // pub(crate) mut self methods
    // --------------------------

    /// Adds a child element by tag and node ID.
    ///
    /// # Arguments
    /// * `child_id` - The node ID of the child element to add.
    /// * `tag` - The tag name of the child element.
    pub(crate) fn add_child_mut(
        &mut self,
        child_id: NodeId,
        tag: &str,
        tag_ns: &str,
    ) -> Result<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }
        // Add the child element to the contents collection
        self.child_contents
            .as_mut()
            .context("Failed to insert child element")?
            .push(XmlElementContentType::Element((
                child_id,
                tag.to_owned(),
                tag_ns.to_owned(),
            )));
        Ok(())
    }

    pub(crate) fn add_child_after_tag_mut(
        &mut self,
        child_id: NodeId,
        new_tag: &str,
        new_tag_ns: &str,
        after_tag: &str,
    ) -> Result<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }

        // Add the child element to the contents collection
        let child_collection = self
            .child_contents
            .as_mut()
            .context("Failed to insert child element")?;
        let last_id = child_collection
            .iter()
            .enumerate()
            .filter_map(|(index, content_type)| match content_type {
                XmlElementContentType::Element((_, tag, _)) => {
                    if tag == after_tag {
                        Some(index)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<usize>>();
        if let Some(last_id) = last_id.last() {
            if (last_id + 1) >= child_collection.len() {
                child_collection.push(XmlElementContentType::Element((
                    child_id,
                    new_tag.to_owned(),
                    new_tag_ns.to_owned(),
                )));
            } else {
                child_collection.insert(
                    *last_id + 1,
                    XmlElementContentType::Element((
                        child_id,
                        new_tag.to_owned(),
                        new_tag_ns.to_owned(),
                    )),
                );
            }
        } else {
            // Add Element at end
            child_collection.push(XmlElementContentType::Element((
                child_id,
                new_tag.to_owned(),
                new_tag_ns.to_owned(),
            )));
        }
        Ok(())
    }

    pub(crate) fn add_child_before_tag_mut(
        &mut self,
        child_id: NodeId,
        new_tag: &str,
        new_tag_ns: &str,
        before_tag: &str,
    ) -> Result<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }
        // Add the child element to the contents collection
        let child_collection = self
            .child_contents
            .as_mut()
            .context("Failed to insert child element")?;
        let first_id = child_collection
            .iter()
            .enumerate()
            .filter_map(|(index, content_type)| match content_type {
                XmlElementContentType::Element((_, tag, _)) => {
                    if tag == before_tag {
                        Some(index)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<usize>>();
        if let Some(first_id) = first_id.first() {
            child_collection.insert(
                *first_id as usize,
                XmlElementContentType::Element((
                    child_id,
                    new_tag.to_owned(),
                    new_tag_ns.to_owned(),
                )),
            );
        } else {
            // Add Element at end
            child_collection.insert(
                0,
                XmlElementContentType::Element((
                    child_id,
                    new_tag.to_owned(),
                    new_tag_ns.to_owned(),
                )),
            );
        }
        Ok(())
    }

    pub(crate) fn add_child_after_tag_ns_mut(
        &mut self,
        child_id: NodeId,
        new_tag: &str,
        new_tag_ns: &str,
        after_tag_ns: &str,
    ) -> Result<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }

        // Add the child element to the contents collection
        let child_collection = self
            .child_contents
            .as_mut()
            .context("Failed to insert child element")?;
        let last_id = child_collection
            .iter()
            .enumerate()
            .filter_map(|(index, content_type)| match content_type {
                XmlElementContentType::Element((_, _, tag_ns)) => {
                    if tag_ns == after_tag_ns {
                        Some(index)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<usize>>();
        if let Some(last_id) = last_id.last() {
            if (last_id + 1) >= child_collection.len() {
                child_collection.push(XmlElementContentType::Element((
                    child_id,
                    new_tag.to_owned(),
                    new_tag_ns.to_owned(),
                )));
            } else {
                child_collection.insert(
                    *last_id + 1,
                    XmlElementContentType::Element((
                        child_id,
                        new_tag.to_owned(),
                        new_tag_ns.to_owned(),
                    )),
                );
            }
        } else {
            // Add Element at end
            child_collection.push(XmlElementContentType::Element((
                child_id,
                new_tag.to_owned(),
                new_tag_ns.to_owned(),
            )));
        }
        Ok(())
    }

    pub(crate) fn add_child_before_tag_ns_mut(
        &mut self,
        child_id: NodeId,
        new_tag: &str,
        new_tag_ns: &str,
        before_tag_ns: &str,
    ) -> Result<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }
        // Add the child element to the contents collection
        let child_collection = self
            .child_contents
            .as_mut()
            .context("Failed to insert child element")?;
        let first_id = child_collection
            .iter()
            .enumerate()
            .filter_map(|(index, content_type)| match content_type {
                XmlElementContentType::Element((_, _, tag_ns)) => {
                    if tag_ns == before_tag_ns {
                        Some(index)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<usize>>();
        if let Some(first_id) = first_id.first() {
            child_collection.insert(
                *first_id as usize,
                XmlElementContentType::Element((
                    child_id,
                    new_tag.to_owned(),
                    new_tag_ns.to_owned(),
                )),
            );
        } else {
            // Add Element at end
            child_collection.insert(
                0,
                XmlElementContentType::Element((
                    child_id,
                    new_tag.to_owned(),
                    new_tag_ns.to_owned(),
                )),
            );
        }
        Ok(())
    }

    /// Sets the node ID of this element.
    ///
    /// # Arguments
    /// * `id` - The NodeId to assign to this element.
    pub(crate) fn set_id_mut(&mut self, id: NodeId) {
        self.id = id;
    }

    /// Sets the parent ID of this element.
    ///
    /// # Arguments
    /// * `parent_id` - The NodeId of the parent element.
    pub(crate) fn set_parent_id_mut(&mut self, parent_id: NodeId) {
        self.parent_id = Some(parent_id);
    }

    /// Clears all content (children, text, comments) from this element.
    pub(crate) fn clear_content_mut(&mut self) {
        self.child_contents = None;
    }

    /// Adds content (child, text, or comment) to this element.
    ///
    /// # Arguments
    /// * `content_type` - The content to add.
    ///
    /// # Returns
    /// * `&mut Self` - For method chaining.
    pub(crate) fn add_child_content_mut(
        &mut self,
        content_type: XmlElementContentType,
    ) -> Result<(), AnyError> {
        // Ensure contents vector exists before adding content
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }
        // Add the content to the contents collection
        self.child_contents
            .as_mut()
            .context("Failed to insert content item")?
            .push(content_type);
        Ok(())
    }

    // --------------------------
    // pub(crate) self methods
    // --------------------------

    /// Gets a reference to the element's attributes.
    ///
    /// # Returns
    /// * `&Option<Vec<XmlAttribute>>` - The attributes, if any.
    pub(crate) fn get_attributes(&self) -> &Option<Vec<XmlAttribute>> {
        &self.attributes
    }

    /// Gets the namespace context for this element.
    ///
    /// # Returns
    /// * `Rc<RefCell<XmlNamespace>>` - Reference-counted pointer to the namespace context.
    pub(crate) fn get_ns_context(&self) -> Rc<RefCell<XmlNamespace>> {
        self.namespace_context.clone()
    }

    /// Creates a limited clone of the element for internal use.
    /// Restricted to crate as it could disrupt the tree handling system if used externally.
    ///
    /// # Returns
    /// * `XmlElement` - A clone of this element.
    pub(crate) fn clone_limited(&self) -> XmlElement {
        XmlElement {
            id: self.id,
            tag: self.tag.clone(),
            child_contents: self.child_contents.clone(),
            parent_id: self.parent_id.clone(),
            ns_alias: self.ns_alias.clone(),
            attributes: self.attributes.clone(),
            ns_context_override: self.ns_context_override,
            namespace_context: self.namespace_context.clone(),
        }
    }

    /// Checks if this element has an attribute with the given name and value.
    ///
    /// # Arguments
    /// * `attr_name` - The name of the attribute to check.
    /// * `attr_value` - The expected value of the attribute.
    ///
    /// # Returns
    /// * `bool` - True if the element has an attribute with the given name and value.
    pub(crate) fn has_attribute(&self, attr_name: &str, attr_value: &str) -> bool {
        if let Some(attributes) = &self.attributes {
            // Check if any attribute matches both name and value
            attributes
                .iter()
                .any(|a| a.get_name() == attr_name && a.get_value() == attr_value)
        } else {
            false
        }
    }

    /// Checks if this element has an attribute with the given name and value.
    ///
    /// # Arguments
    /// * `attr_name` - The name of the attribute to check.
    /// * `attr_value` - The expected value of the attribute.
    ///
    /// # Returns
    /// * `bool` - True if the element has an attribute with the given name and value.
    pub(crate) fn has_attribute_ns(&self, attr_name_ns: &str, attr_value: &str) -> bool {
        if let Some(attributes) = &self.attributes {
            // Check if any attribute matches both name and value
            attributes
                .iter()
                .any(|a| a.get_ns_name() == attr_name_ns && a.get_value() == attr_value)
        } else {
            false
        }
    }

    /// Checks if this element has a namespace.
    ///
    /// # Returns
    /// - `bool` - True if the element has a namespace, false otherwise.
    pub(crate) fn has_namespace(&self) -> bool {
        self.ns_context_override
    }

    /// Returns a reference to the namespace context for this element.
    pub(crate) fn get_namespace_context(&self) -> Rc<RefCell<XmlNamespace>> {
        self.namespace_context.clone()
    }
}

impl XmlElement {
    // --------------------------
    // Public constructor
    // --------------------------

    /// Creates a new XML element with the given tag, attributes, and namespace context.
    ///
    /// # Arguments
    /// * `tag` - The tag name for the element, possibly with namespace prefix.
    /// * `attributes` - Optional attributes for the element.
    /// * `namespace_context` - The namespace context for resolving prefixes.
    ///
    /// # Returns
    /// * `Result<XmlElement, AnyError>` - A new element or an error if validation fails.
    pub(crate) fn new(
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
        mut namespace_context: Rc<RefCell<XmlNamespace>>,
    ) -> Result<XmlElement, AnyError> {
        let mut ns_context_override = false;

        // Validate that the tag name follows XML naming rules
        if validation::is_valid_xml_name(&tag) {
            // Validate that all attribute names follow XML naming rules
            let filtered_attributes = if let Some(mut attributes) = attributes {
                if !attributes
                    .iter()
                    .all(|attribute| validation::is_valid_xml_name(&attribute.get_ns_name()))
                {
                    return Err(AnyError::msg("Not all attributes satisfy naming standards"));
                }

                // Process namespace declarations (xmlns attributes)
                let mut namespaces = Vec::new();

                // Extract namespace declarations from attributes
                attributes.retain(|attribute| {
                    if attribute.get_ns_name().starts_with("xmlns") {
                        namespaces.push(attribute.clone());
                        false // Remove from regular attributes
                    } else {
                        true // Keep as regular attribute
                    }
                });

                // If namespace declarations found, create a new namespace context
                if !namespaces.is_empty() {
                    ns_context_override = true;
                    namespace_context = Rc::new(RefCell::new(XmlNamespace::new()));

                    // Add each namespace declaration to the context
                    for namespace in namespaces {
                        namespace_context.borrow_mut().add_namespace_mut(namespace);
                    }
                }
                Some(attributes)
            } else {
                None
            };

            // Parse the tag for namespace prefix
            let (ns_alias, tag) = if let Some(pos) = tag.find(':') {
                let (ns, tag) = tag.split_at(pos);
                (Some(ns.to_string()), &tag[1..])
            } else {
                (None, tag)
            };

            // Create and return the new element
            Ok(XmlElement {
                id: 0, // Initial ID, will be set by document
                tag: tag.to_owned(),
                attributes: filtered_attributes,
                parent_id: None,
                child_contents: None,
                ns_alias,
                ns_context_override,
                namespace_context,
            })
        } else {
            Err(AnyError::msg("Invalid XML tag name"))
        }
    }
}
