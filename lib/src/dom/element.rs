/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{utils::validation::is_valid_xml_name, NodeId, XmlAttribute, XmlNamespace};
use anyhow::{Context, Error as AnyError, Result as AnyResult};
use std::{cell::RefCell, rc::Rc};

/// Element tag name without namespace suppor
pub type Tag = String;
/// Element tag name with namespace suppor
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

// Consumer Public mut API
impl XmlElement {
    // --------------------------
    // pub mut self methods
    // --------------------------

    /// Adds an attribute to this element.
    ///
    /// The attribute name must be unique within the element. If an attribute with the
    /// same namespaced name already exists, an error is returned.
    ///
    /// # Arguments
    /// * `attribute` - The XML attribute to add to this element.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - An empty result if the attribute was added successfully,
    ///   or an error if an attribute with the same name already exists.
    pub fn add_attribute_mut(&mut self, attribute: XmlAttribute) -> AnyResult<(), AnyError> {
        let attributes = self.attributes.get_or_insert_with(Vec::new);
        // Validate ns alias if exist
        if !attribute.is_valid_ns_alias(self.namespace_context.clone()) {
            return Err(AnyError::msg(
                "draviavemal-xml_rs::Add attribute namespace alias used without refering schema",
            ));
        }
        // Reject duplicate attribute names to keep them unique per element
        if attributes
            .iter()
            .any(|existing_attribute| existing_attribute.get_ns_name() == attribute.get_ns_name())
        {
            return Err(AnyError::msg(format!(
                "draviavemal-xml_rs::Attribute '{}' already exists on this element",
                attribute.get_ns_name()
            )));
        }
        // Add the attribute to the attributes collection
        attributes.push(attribute);
        Ok(())
    }

    /// Adds or replaces an attribute on this element.
    ///
    /// The attribute is matched by its namespaced name. If an attribute with the same
    /// name already exists, it is replaced in-place at its current position. Otherwise,
    /// the attribute is appended to the end of the attribute list.
    ///
    /// # Arguments
    /// * `attribute` - The XML attribute to add or replace.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Ok on success.
    pub fn add_replace_attribute_mut(
        &mut self,
        attribute: XmlAttribute,
    ) -> AnyResult<(), AnyError> {
        let attributes = self.attributes.get_or_insert_with(Vec::new);
        // Validate ns alias if exist
        if !attribute.is_valid_ns_alias(self.namespace_context.clone()) {
            return Err(AnyError::msg(
                "draviavemal-xml_rs::Replace attribute namespace alias used without refering schema",
            ));
        }
        // Locate the existing attribute by its namespaced name
        let existing_index = attributes.iter().position(|existing_attribute| {
            existing_attribute.get_ns_name() == attribute.get_ns_name()
        });
        match existing_index {
            // Replace in-place at the existing position
            Some(index) => attributes[index] = attribute,
            // Otherwise append to the end
            None => attributes.push(attribute),
        }
        Ok(())
    }

    /// Sets the initial attributes of this element from a vector.
    ///
    /// This is intended for initialising an element's attributes. If the element already
    /// has one or more attributes, an error is returned.
    ///
    /// # Arguments
    /// * `attributes` - The attributes to set on this element.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Ok on success, or an error if the element already
    ///   has attributes.
    pub fn set_attribute_mut(&mut self, attributes: Vec<XmlAttribute>) -> AnyResult<(), AnyError> {
        // Only allow setting when there are no existing attributes
        if self.attributes.is_some() {
            return Err(AnyError::msg(
                "draviavemal-xml_rs::Element already has attributes; cannot set initial attributes",
            ));
        }
        // Validate attribute NS
        if !attributes
            .iter()
            .all(|attribute| attribute.is_valid_ns_alias(self.namespace_context.clone()))
        {
            return Err(AnyError::msg(
                "draviavemal-xml_rs::Set attribute namespace alias used without refering schema",
            ));
        }
        self.attributes = Some(attributes);
        Ok(())
    }

    /// Clear all attributes of this element.
    ///
    /// # Returns
    /// * `AnyResult<u32, AnyError>` - The number of attributes that were removed.
    pub fn clear_attribute_mut(&mut self) -> AnyResult<u32, AnyError> {
        let removed_count = self.attributes.iter().flatten().count() as u32;
        self.attributes = None;
        Ok(removed_count)
    }

    /// Removes an attribute by its local name.
    /// Caution: This method does not consider namespaces. If multiple attributes share the same local name but different namespaces, all will be removed.
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
    pub fn add_text_mut(&mut self, text: &str) -> AnyResult<(), AnyError> {
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
    /// * `AnyResult<&mut XmlElement, AnyError>` - A mutable reference to self for method chaining,
    ///   or an error if adding the comment failed.
    ///
    pub fn add_comments_mut(&mut self, comment: &str) -> AnyResult<(), AnyError> {
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

    /// Retrieves an attribute by its namespace URI and local name.
    ///
    /// Resolves each prefixed attribute's alias against the element's namespace context
    /// and compares the resolved URI, making this alias-independent.
    pub fn get_attribute_by_uri(&self, uri: &str, local_name: &str) -> Option<&XmlAttribute> {
        let ctx = self.namespace_context.borrow();
        self.attributes.as_ref()?.iter().find(|attr| {
            attr.get_name() == local_name
                && attr
                    .get_ns_alias()
                    .and_then(|alias| ctx._get_url(alias))
                    .map(|resolved| resolved == uri)
                    .unwrap_or(false)
        })
    }

    /// Returns the namespace alias currently in scope for the given URI, if any.
    pub fn get_alias_for_uri(&self, uri: &str) -> Option<String> {
        self.namespace_context.borrow()._get_alias(uri).cloned()
    }

    /// Retrives all attribute keys without namespace
    ///
    /// # Returns
    /// * `Option<Vec<String>>` - A reference to the attribute if found, or None.
    pub fn get_attribute_keys(&self) -> Option<Vec<String>> {
        if let Some(attributes) = self.attributes.as_ref() {
            Some(
                attributes
                    .iter()
                    .map(|item| item.get_name().to_string())
                    .collect::<Vec<String>>(),
            )
        } else {
            None
        }
    }

    /// Retrives all attribute keys with namespace
    ///
    /// # Returns
    /// * `Option<Vec<String>>` - A reference to the attribute if found, or None.
    pub fn get_attribute_ns_keys(&self) -> Option<Vec<String>> {
        if let Some(attributes) = self.attributes.as_ref() {
            Some(
                attributes
                    .iter()
                    .map(|item| item.get_ns_name().to_string())
                    .collect::<Vec<String>>(),
            )
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
    /// * `AnyResult<u32, AnyError>` - The count of child elements, or an error if the contents are not accessible.
    pub fn get_child_element_count(&self) -> AnyResult<u32, AnyError> {
        let count = self
            .child_contents
            .as_ref()
            .context("draviavemal-xml_rs::Failed to open contents")?
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

    /// Get the Text value of the element if exist else return none
    ///
    /// # Returns
    /// * `AnyResult<Option<String>, AnyError>` - Result chain to read just string value of element
    pub fn get_element_text_value(&self) -> AnyResult<Option<String>, AnyError> {
        for content in self
            .get_child_contents()
            .as_ref()
            .context("draviavemal-xml_rs::Failed to get content Childs")?
        {
            match content {
                XmlElementContentType::Text(value) => return Ok(Some(value.clone())),
                _ => {}
            }
        }
        Ok(None)
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
    ) -> AnyResult<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }
        // Add the child element to the contents collection
        self.child_contents
            .as_mut()
            .context("draviavemal-xml_rs::Failed to insert child element")?
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
    ) -> AnyResult<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }

        // Add the child element to the contents collection
        let child_collection = self
            .child_contents
            .as_mut()
            .context("draviavemal-xml_rs::Failed to insert child element")?;
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
    ) -> AnyResult<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }
        // Add the child element to the contents collection
        let child_collection = self
            .child_contents
            .as_mut()
            .context("draviavemal-xml_rs::Failed to insert child element")?;
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
    ) -> AnyResult<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }

        // Add the child element to the contents collection
        let child_collection = self
            .child_contents
            .as_mut()
            .context("draviavemal-xml_rs::Failed to insert child element")?;
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
    ) -> AnyResult<(), AnyError> {
        // Ensure contents vector exists before adding a new child
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }
        // Add the child element to the contents collection
        let child_collection = self
            .child_contents
            .as_mut()
            .context("draviavemal-xml_rs::Failed to insert child element")?;
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
    ) -> AnyResult<(), AnyError> {
        // Ensure contents vector exists before adding content
        if self.child_contents.is_none() {
            self.child_contents = Some(Vec::new());
        }
        // Add the content to the contents collection
        self.child_contents
            .as_mut()
            .context("draviavemal-xml_rs::Failed to insert content item")?
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
    /// * `AnyResult<XmlElement, AnyError>` - A new element or an error if validation fails.
    pub(crate) fn new(
        new_tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
        mut namespace_context: Rc<RefCell<XmlNamespace>>,
    ) -> AnyResult<XmlElement, AnyError> {
        let mut ns_context_override = false;

        // Validate that the tag name follows XML naming rules
        if is_valid_xml_name(&new_tag) {
            // Validate that all attribute names follow XML naming rules
            let filtered_attributes = if let Some(mut attributes) = attributes {
                if !attributes
                    .iter()
                    .all(|attribute| is_valid_xml_name(&attribute.get_ns_name()))
                {
                    return Err(AnyError::msg(
                        "draviavemal-xml_rs::Not all attributes satisfy naming standards",
                    ));
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

                // If namespace declarations found, inherit parent scope then layer new declarations on top
                if !namespaces.is_empty() {
                    ns_context_override = true;
                    let inherited = (*namespace_context.borrow()).clone();
                    namespace_context = Rc::new(RefCell::new(inherited));

                    // Add each namespace declaration to the context
                    for namespace in namespaces {
                        namespace_context.borrow_mut().add_namespace_mut(namespace);
                    }
                }
                if attributes.len() == 0 {
                    None
                } else {
                    Some(attributes)
                }
            } else {
                None
            };

            // Parse the tag for namespace prefix
            let (ns_alias, tag) = if let Some(pos) = new_tag.find(':') {
                let (ns, tag) = new_tag.split_at(pos);
                // Validate namespace alias is declared
                if !namespace_context
                    .try_borrow()
                    .context("draviavemal-xml_rs::Failed to fetch Namespace context")?
                    .is_valid_ns_alias(ns)
                {
                    return Err(AnyError::msg(format!(
                        "draviavemal-xml_rs::Tag namespace alias used without refering schema {}:{}",
                        ns,
                        &tag[1..]
                    )));
                }
                (Some(ns.to_string()), &tag[1..])
            } else {
                (None, new_tag)
            };

            // Validate attribute NS
            if filtered_attributes.is_some()
                && !filtered_attributes
                    .as_ref()
                    .context("draviavemal-xml_rs::Failed to read attribute")?
                    .iter()
                    .all(|attribute| attribute.is_valid_ns_alias(namespace_context.clone()))
            {
                return Err(AnyError::msg(
                    "draviavemal-xml_rs::Attribute in new tag namespace alias used without refering schema",
                ));
            }

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
            Err(AnyError::msg("draviavemal-xml_rs::draviavemal-xml_rs::Invalid XML tag name"))
        }
    }
}
