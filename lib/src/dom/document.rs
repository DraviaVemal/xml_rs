/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{
    NamespaceDeclaration, NsTag, Tag, XPathHandler, XmlAttribute, XmlElement,
    XmlElementContentType, XmlNamespace,
};
use anyhow::{Context, Error as AnyError};
use log::{debug, trace, warn};
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
    /// XML `standalone` declaration value, when present
    standalone: Option<String>,
    /// Comments appearing in the prolog, before the root element
    prolog_comments: Vec<String>,
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
            standalone: None,
            prolog_comments: Vec::new(),
            running_id: 0,
            root_id: 1,
            xml_element_collection: BTreeMap::new(),
        }
    }
}

impl XmlDocument {
    /// Resolves the alias for `ns_declaration` in an element's scope, declaring it if missing.
    pub fn resolve_alias_mut(
        &mut self,
        element_id: NodeId,
        ns_declaration: &NamespaceDeclaration,
    ) -> Result<String, AnyError> {
        Ok(self
            .get_element_mut(element_id)
            .context("draviavemal-xml_rs::Element not found for alias resolution")?
            .resolve_alias_mut(ns_declaration))
    }

    /// Prefer [`XmlDocument::create_root_element_ns_mut`] for round-trippable namespaces.
    ///
    /// Creates the root element from a raw `tag`; any prefix must be declared through an
    /// accompanying `xmlns` attribute in `attributes`.
    pub fn create_root_element_mut(
        &mut self,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        self.insert_root_element(tag, attributes)
    }

    /// Creates the root element for `local_name`, emitting the alias from `ns_declaration` as an
    /// `xmlns` declaration (the root always starts from an empty scope).
    pub fn create_root_element_ns_mut(
        &mut self,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let alias = ns_declaration
            .alias_override
            .unwrap_or(ns_declaration.default_alias);
        let (root_tag, xmlns_name) = if alias.is_empty() {
            (local_name.to_owned(), "xmlns".to_owned())
        } else {
            (
                format!("{}:{}", alias, local_name),
                format!("xmlns:{}", alias),
            )
        };
        let mut augmented_attributes = attributes.unwrap_or_default();
        augmented_attributes.push(XmlAttribute::new(xmlns_name, ns_declaration.uri.to_owned()));
        self.insert_root_element(&root_tag, Some(augmented_attributes))
    }

    /// Prefer [`XmlDocument::append_child_element_ns_mut`] for round-trippable namespaces.
    ///
    /// Appends a child from a raw `tag`; any prefix must already be declared in the parent
    /// scope or introduced via an `xmlns` attribute in `attributes`.
    pub fn append_child_element_mut(
        &mut self,
        parent_id: NodeId,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        self.insert_child_element(parent_id, tag, attributes)
    }

    /// Appends a child `local_name` whose alias is resolved from `ns_declaration`, declaring the
    /// namespace on the child only when it is not already in the parent scope.
    pub fn append_child_element_ns_mut(
        &mut self,
        parent_id: NodeId,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (ns_tag, attributes) =
            self.build_ns_child_tag(parent_id, ns_declaration, local_name, attributes)?;
        self.insert_child_element(parent_id, &ns_tag, attributes)
    }

    /// Prefer [`XmlDocument::insert_child_element_after_last_tag_ns_mut`] for round-trippable
    /// namespaces.
    ///
    /// Inserts a child from raw `tag` after the last `after_tag`; a prefixed `after_tag` is
    /// matched by namespaced name, otherwise by local name. Falls back to appending.
    pub fn insert_child_element_after_last_tag_mut(
        &mut self,
        parent_id: NodeId,
        tag: &str,
        after_tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let match_by_ns = after_tag.contains(':');
        self.insert_child_after(
            parent_id,
            tag,
            attributes,
            after_tag.to_owned(),
            match_by_ns,
        )
    }

    /// Inserts a child `local_name` (resolved from `ns_declaration`) after the last element
    /// matching `after_local_name` resolved through `after_ns_declaration`.
    pub fn insert_child_element_after_last_tag_ns_mut(
        &mut self,
        parent_id: NodeId,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
        after_local_name: &str,
        after_ns_declaration: &NamespaceDeclaration,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (ns_tag, attributes) =
            self.build_ns_child_tag(parent_id, ns_declaration, local_name, attributes)?;
        let after_reference =
            self.build_reference_ns_tag(parent_id, after_local_name, after_ns_declaration)?;
        self.insert_child_after(parent_id, &ns_tag, attributes, after_reference, true)
    }

    /// Prefer [`XmlDocument::insert_child_element_before_first_tag_ns_mut`] for round-trippable
    /// namespaces.
    ///
    /// Inserts a child from raw `tag` before the first `before_tag`; a prefixed `before_tag` is
    /// matched by namespaced name, otherwise by local name. Falls back to inserting at the front.
    pub fn insert_child_element_before_first_tag_mut(
        &mut self,
        parent_id: NodeId,
        tag: &str,
        before_tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let match_by_ns = before_tag.contains(':');
        self.insert_child_before(
            parent_id,
            tag,
            attributes,
            before_tag.to_owned(),
            match_by_ns,
        )
    }

    /// Inserts a child `local_name` (resolved from `ns_declaration`) before the first element
    /// matching `before_local_name` resolved through `before_ns_declaration`.
    pub fn insert_child_element_before_first_tag_ns_mut(
        &mut self,
        parent_id: NodeId,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
        before_local_name: &str,
        before_ns_declaration: &NamespaceDeclaration,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (ns_tag, attributes) =
            self.build_ns_child_tag(parent_id, ns_declaration, local_name, attributes)?;
        let before_reference =
            self.build_reference_ns_tag(parent_id, before_local_name, before_ns_declaration)?;
        self.insert_child_before(parent_id, &ns_tag, attributes, before_reference, true)
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
            .with_context(|| {
                warn!(
                    "draviavemal-xml_rs::Mutable element lookup failed for node id {}",
                    active_xml_element_id
                );
                "draviavemal-xml_rs::Get Element mut not found"
            })
    }

    /// Clears the content of an element, removing all of its children.
    ///
    /// # Arguments
    /// * `element_id` - The node ID of the element to clear.
    ///
    /// # Returns
    /// * `Result<(), AnyError>` - Success or an error.
    pub fn clear_element_content_mut(&mut self, element_id: NodeId) -> Result<(), AnyError> {
        debug!(
            "draviavemal-xml_rs::Clearing content of element node {}",
            element_id
        );
        // Remove all child elements from the document
        self.clear_element_subtree_mut(element_id)?;

        // Clear the element's content
        self.get_element_mut(element_id)
            .context("draviavemal-xml_rs::Failed to get element")?
            .clear_content_mut();

        Ok(())
    }
}

impl XmlDocument {
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

    /// Gets the XML `standalone` declaration value, if present.
    ///
    /// # Returns
    /// * `Option<&str>` - The standalone value (e.g., "yes"), or None when not declared.
    pub fn get_standalone(&self) -> Option<&str> {
        self.standalone.as_deref()
    }

    /// Gets the comments declared in the prolog, before the root element.
    ///
    /// # Returns
    /// * `&[String]` - The prolog comments in document order.
    pub fn get_prolog_comments(&self) -> &[String] {
        &self.prolog_comments
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
            .with_context(|| {
                warn!(
                    "draviavemal-xml_rs::Element lookup failed for node id {}",
                    active_xml_element_id
                );
                "draviavemal-xml_rs::Get Element not found"
            })
    }

    /// Creates a clone of the document.
    ///
    /// # Returns
    /// * `XmlDocument` - A new document with the same content.
    pub fn clone(&self) -> XmlDocument {
        XmlDocument {
            version: self.version.clone(),
            encoding: self.encoding.clone(),
            standalone: self.standalone.clone(),
            prolog_comments: self.prolog_comments.clone(),
            running_id: self.running_id,
            root_id: self.root_id,
            xml_element_collection: self
                .xml_element_collection
                .iter()
                .map(|(node_id, element)| (*node_id, element.clone_limited()))
                .collect(),
        }
    }

    /// Evaluates an XPath-style query against the document.
    ///
    /// Note: XPath evaluation is not yet implemented; this currently parses the query and
    /// returns `Ok(None)`.
    ///
    /// # Arguments
    /// * `query_path` - The query expression to evaluate.
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - The matching node IDs, or None when there
    ///   are no matches.
    pub fn query_xpath(&self, query_path: &str) -> Result<Option<Vec<NodeId>>, AnyError> {
        let xpath_handler = XPathHandler::new(query_path);
        Ok(None)
    }

    /// Prefer [`XmlDocument::find_first_child_ns`] for round-trippable namespaces.
    ///
    /// Finds the first child element matching `tag`; a prefixed `tag` is matched by its
    /// namespaced name, otherwise by local name.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `tag` - The tag name to search for, optionally namespaced (e.g., "ns:tag").
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
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .find_first_child(tag))
    }

    /// Finds the first child element whose alias is resolved from `ns_declaration` in the parent
    /// scope and whose local name matches `local_name`.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `local_name` - The local tag name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the tag's namespace.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The ID of the first matching child, or None if not found.
    pub fn find_first_child_ns(
        &self,
        parent_id: NodeId,
        tag: &str,
        ns_declaration: &NamespaceDeclaration,
    ) -> Result<Option<NodeId>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .find_first_child_ns(tag, ns_declaration))
    }

    /// Prefer [`XmlDocument::find_all_child_ns`] for round-trippable namespaces.
    ///
    /// Finds all child elements matching `tag`; a prefixed `tag` is matched by its namespaced
    /// name, otherwise by local name.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `tag` - The tag name to search for, optionally namespaced (e.g., "ns:tag").
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
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .find_all_child(tag))
    }

    /// Finds all child elements whose alias is resolved from `ns_declaration` in the parent
    /// scope and whose local name matches `local_name`.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `local_name` - The local tag name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the tag's namespace.
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - A vector of matching child IDs, or None if none found.
    pub fn find_all_child_ns(
        &self,
        parent_id: NodeId,
        tag: &str,
        ns_declaration: &NamespaceDeclaration,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .find_all_child_ns(tag, ns_declaration))
    }

    /// Prefer [`XmlDocument::find_first_by_attribute_ns`] for round-trippable namespaces.
    ///
    /// Finds the first child element with a matching attribute; a prefixed `attr_name` is
    /// matched by its namespaced name, otherwise by local name.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `attr_name` - The attribute name to match, optionally namespaced (e.g., "ns:attr").
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
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_child_contents()
        {
            // Iterate through each content item
            for content in contents {
                if let XmlElementContentType::Element((child_id, _, _)) = content {
                    // Check if the child element has the specified attribute with the specified value
                    if self
                        .get_element(*child_id)
                        .context("draviavemal-xml_rs::Failed to pull child element")?
                        .attribute_value_matches(attr_name, attr_value)
                    {
                        return Ok(Some(child_id.clone()));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Finds the first child element whose attribute alias is resolved from `ns_declaration`
    /// in the parent scope, matching `attr_name` and `attr_value`.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `attr_name` - The attribute local name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the attribute's namespace.
    /// * `attr_value` - The attribute value to match.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The ID of the first matching child, or None if not found.
    pub fn find_first_by_attribute_ns(
        &self,
        parent_id: NodeId,
        attr_name: &str,
        ns_declaration: &NamespaceDeclaration,
        attr_value: &str,
    ) -> Result<Option<NodeId>, AnyError> {
        if let Some(contents) = self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_child_contents()
        {
            for content in contents {
                if let XmlElementContentType::Element((child_id, _, _)) = content {
                    if self
                        .get_element(*child_id)
                        .context("draviavemal-xml_rs::Failed to pull child element")?
                        .get_attribute_value_ns(attr_name, ns_declaration)
                        == Some(attr_value)
                    {
                        return Ok(Some(*child_id));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Prefer [`XmlDocument::find_all_by_attribute_ns`] for round-trippable namespaces.
    ///
    /// Finds all child elements with a matching attribute; a prefixed `attr_name` is matched by
    /// its namespaced name, otherwise by local name.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `attr_name` - The attribute name to match, optionally namespaced (e.g., "ns:attr").
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
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_child_contents()
        {
            // Iterate through each content item
            for content in contents {
                if let XmlElementContentType::Element((child_id, _, _)) = content {
                    // Check if the child element has the specified attribute with the specified value
                    if self
                        .get_element(*child_id)
                        .context("draviavemal-xml_rs::Failed to pull child element")?
                        .attribute_value_matches(attr_name, attr_value)
                    {
                        result.push(*child_id);
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

    /// Finds all child elements whose attribute alias is resolved from `ns_declaration` in the
    /// parent scope, matching `attr_name` and `attr_value`.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `attr_name` - The attribute local name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the attribute's namespace.
    /// * `attr_value` - The attribute value to match.
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - A vector of matching child IDs, or None if none found.
    pub fn find_all_by_attribute_ns(
        &self,
        parent_id: NodeId,
        attr_name: &str,
        ns_declaration: &NamespaceDeclaration,
        attr_value: &str,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        let mut result = Vec::new();

        if let Some(contents) = self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_child_contents()
        {
            for content in contents {
                if let XmlElementContentType::Element((child_id, _, _)) = content {
                    if self
                        .get_element(*child_id)
                        .context("draviavemal-xml_rs::Failed to pull child element")?
                        .get_attribute_value_ns(attr_name, ns_declaration)
                        == Some(attr_value)
                    {
                        result.push(*child_id);
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

    /// Gets the parent element ID of the given element.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the element whose parent is requested.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The parent ID, or None for the root element.
    pub fn get_parent(&self, element_id: NodeId) -> Result<Option<NodeId>, AnyError> {
        Ok(self
            .get_element(element_id)
            .context("draviavemal-xml_rs::Failed to pull element")?
            .get_parent_id())
    }

    /// Gets the IDs of all direct child elements of the given element, in document order.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - The child element IDs, or None when there are none.
    pub fn get_children(&self, parent_id: NodeId) -> Result<Option<Vec<NodeId>>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_child_element_ids())
    }

    /// Gets the ID of the first direct child element of the given element.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The first child element ID, or None when there are none.
    pub fn get_first_child_element(&self, parent_id: NodeId) -> Result<Option<NodeId>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_first_child_element())
    }

    /// Gets the ID of the last direct child element of the given element.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The last child element ID, or None when there are none.
    pub fn get_last_child_element(&self, parent_id: NodeId) -> Result<Option<NodeId>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_last_child_element())
    }

    /// Gets the next sibling element that follows the given element under the same parent.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the reference element.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The following sibling ID, or None when it is last.
    pub fn get_next_sibling(&self, element_id: NodeId) -> Result<Option<NodeId>, AnyError> {
        self.sibling_at_offset(element_id, 1)
    }

    /// Gets the previous sibling element that precedes the given element under the same parent.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the reference element.
    ///
    /// # Returns
    /// * `Result<Option<NodeId>, AnyError>` - The preceding sibling ID, or None when it is first.
    pub fn get_previous_sibling(&self, element_id: NodeId) -> Result<Option<NodeId>, AnyError> {
        self.sibling_at_offset(element_id, -1)
    }

    /// Prefer [`XmlDocument::get_elements_by_tag_name_ns`] for round-trippable namespaces.
    ///
    /// Collects all descendant elements matching `tag`; a prefixed `tag` is matched by its
    /// namespaced name, otherwise by local name. The search spans the whole subtree.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the element whose subtree is searched.
    /// * `tag` - The tag name to search for, optionally namespaced (e.g., "ns:tag").
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - The matching descendant IDs, or None if none found.
    pub fn get_elements_by_tag_name(
        &self,
        parent_id: NodeId,
        tag: &str,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        let match_by_ns = tag.contains(':');
        let mut result = Vec::new();
        self.collect_descendants_by_tag(parent_id, tag, match_by_ns, &mut result)?;
        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    /// Collects all descendant elements whose alias is resolved from `ns_declaration` in the
    /// subtree root's scope and whose local name matches `local_name`.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the element whose subtree is searched.
    /// * `local_name` - The local tag name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the tag's namespace.
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>>, AnyError>` - The matching descendant IDs, or None if none found.
    pub fn get_elements_by_tag_name_ns(
        &self,
        parent_id: NodeId,
        tag: &str,
        ns_declaration: &NamespaceDeclaration,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        let tag_ns = self.build_reference_ns_tag(parent_id, tag, ns_declaration)?;
        let mut result = Vec::new();
        self.collect_descendants_by_tag(parent_id, &tag_ns, true, &mut result)?;
        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    /// Concatenates the text of every descendant text node of the given element.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the element whose text content is collected.
    ///
    /// # Returns
    /// * `Result<String, AnyError>` - The joined descendant text.
    pub fn get_element_text_content(&self, element_id: NodeId) -> Result<String, AnyError> {
        let mut text = String::new();
        self.collect_text_content(element_id, &mut text)?;
        Ok(text)
    }

    /// Resolves the namespace URI bound to `prefix` in the given element's scope.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the element providing the scope.
    /// * `prefix` - The namespace prefix to resolve, empty for the default namespace.
    ///
    /// # Returns
    /// * `Result<Option<String>, AnyError>` - The bound URI, or None when the prefix is undeclared.
    pub fn lookup_namespace_uri(
        &self,
        element_id: NodeId,
        prefix: &str,
    ) -> Result<Option<String>, AnyError> {
        Ok(self
            .get_element(element_id)
            .context("draviavemal-xml_rs::Failed to pull element")?
            .resolve_alias_to_uri(prefix))
    }

    /// Resolves the prefix bound to `uri` in the given element's scope.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the element providing the scope.
    /// * `uri` - The namespace URI to resolve.
    ///
    /// # Returns
    /// * `Result<Option<String>, AnyError>` - The in-scope prefix, or None when the URI is undeclared.
    pub fn lookup_prefix(&self, element_id: NodeId, uri: &str) -> Result<Option<String>, AnyError> {
        Ok(self
            .get_element(element_id)
            .context("draviavemal-xml_rs::Failed to pull element")?
            .get_alias_for_uri(uri))
    }

    /// Replaces the entire content of an element with a single text node.
    ///
    /// Descendant elements are removed from the document before the text is set.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the element to update.
    /// * `text` - The text that becomes the element's only content.
    ///
    /// # Returns
    /// * `Result<(), AnyError>` - Success or an error.
    pub fn set_element_text_mut(&mut self, element_id: NodeId, text: &str) -> Result<(), AnyError> {
        self.clear_element_content_mut(element_id)?;
        self.get_element_mut(element_id)
            .context("draviavemal-xml_rs::Failed to get element")?
            .add_text_mut(text)
    }

    /// Sets the XML version string.
    ///
    /// # Arguments
    /// * `version` - The version string to set (e.g., "1.0").
    pub fn set_version_mut(&mut self, version: String) {
        self.version = version;
    }

    /// Sets the XML document encoding.
    ///
    /// # Arguments
    /// * `encoding` - The encoding string to set (e.g., "UTF-8").
    pub fn set_encoding_mut(&mut self, encoding: String) {
        self.encoding = encoding
    }

    /// Sets the XML `standalone` declaration value.
    ///
    /// # Arguments
    /// * `standalone` - The standalone value to set (e.g., "yes"), or None to omit it.
    pub fn set_standalone_mut(&mut self, standalone: Option<String>) {
        self.standalone = standalone;
    }

    /// Appends a comment to the document prolog, before the root element.
    ///
    /// # Arguments
    /// * `comment` - The comment text without the `<!--` and `-->` delimiters.
    pub fn add_prolog_comment_mut(&mut self, comment: String) {
        self.prolog_comments.push(comment);
    }

    /// Removes an element and all its descendants from the document.
    ///
    /// # Arguments
    /// * `element_id` - The ID of the element to remove.
    ///
    /// # Returns
    /// * `Result<(), AnyError>` - Success or an error.
    pub fn remove_element_mut(&mut self, element_id: NodeId) -> Result<(), AnyError> {
        debug!(
            "draviavemal-xml_rs::Removing element node {} and its subtree",
            element_id
        );
        // Get the parent ID of the element
        if let Some(parent_id) = self
            .get_element_mut(element_id)
            .context("draviavemal-xml_rs::Failed to get element")?
            .get_parent_id()
        {
            // Remove the element from its parent's contents
            if let Some(parent) = self
                .get_element_mut(parent_id)
                .context("draviavemal-xml_rs::Failed to get parent element")?
                .get_child_contents_mut()
            {
                // Filter out the element from parent's contents
                parent.retain(|content| match content {
                    XmlElementContentType::Element((child_id, _, _)) => *child_id != element_id,
                    _ => true,
                });
            }
        }

        if let Ok(element) = self.get_element(element_id) {
            element.release_ns_usage();
        }

        // Remove all descendant elements recursively
        self.clear_element_subtree_mut(element_id)
            .context("draviavemal-xml_rs::Failed to clean up child element tree")?;

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
            .context("draviavemal-xml_rs::Failed to get element")?
            .get_child_contents()
            .clone()
        {
            // Process each content item
            for content in contents {
                if let XmlElementContentType::Element((child_id, _, _)) = content {
                    if let Ok(child) = self.get_element(child_id) {
                        child.release_ns_usage();
                    }
                    // Recursively clear the subtree of each child element
                    self.clear_element_subtree_mut(child_id)?;
                }
            }
        }
        Ok(())
    }
}

impl XmlDocument {
    /// Returns the sibling element `offset` positions from `element_id` under its parent.
    fn sibling_at_offset(
        &self,
        element_id: NodeId,
        offset: i64,
    ) -> Result<Option<NodeId>, AnyError> {
        let parent_id = match self
            .get_element(element_id)
            .context("draviavemal-xml_rs::Failed to pull element")?
            .get_parent_id()
        {
            Some(parent_id) => parent_id,
            None => return Ok(None),
        };
        let siblings = match self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_child_element_ids()
        {
            Some(siblings) => siblings,
            None => return Ok(None),
        };
        let position = match siblings.iter().position(|id| *id == element_id) {
            Some(position) => position as i64,
            None => return Ok(None),
        };
        let target = position + offset;
        if target < 0 || target as usize >= siblings.len() {
            Ok(None)
        } else {
            Ok(Some(siblings[target as usize]))
        }
    }

    /// Recursively collects descendants of `element_id` whose tag matches, honoring `match_by_ns`.
    fn collect_descendants_by_tag(
        &self,
        element_id: NodeId,
        tag: &str,
        match_by_ns: bool,
        result: &mut Vec<NodeId>,
    ) -> Result<(), AnyError> {
        if let Some(contents) = self
            .get_element(element_id)
            .context("draviavemal-xml_rs::Failed to pull element")?
            .get_child_contents()
        {
            for content in contents {
                if let XmlElementContentType::Element((child_id, child_tag, child_tag_ns)) = content
                {
                    let candidate = if match_by_ns {
                        child_tag_ns.as_str()
                    } else {
                        child_tag.as_str()
                    };
                    if candidate == tag {
                        result.push(*child_id);
                    }
                    self.collect_descendants_by_tag(*child_id, tag, match_by_ns, result)?;
                }
            }
        }
        Ok(())
    }

    /// Recursively appends the text of every descendant text node into `text`.
    fn collect_text_content(&self, element_id: NodeId, text: &mut String) -> Result<(), AnyError> {
        if let Some(contents) = self
            .get_element(element_id)
            .context("draviavemal-xml_rs::Failed to pull element")?
            .get_child_contents()
        {
            for content in contents {
                match content {
                    XmlElementContentType::Text(value) => text.push_str(value),
                    XmlElementContentType::Element((child_id, _, _)) => {
                        self.collect_text_content(*child_id, text)?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

impl XmlDocument {
    /// Builds a namespaced child tag from a declaration and augments attributes if needed.
    ///
    /// Resolves the alias for `declaration.uri` against the parent scope and, when the URI
    /// is not already declared, appends the matching `xmlns` attribute so the child element
    /// introduces the binding.
    ///
    /// # Returns
    /// * `Result<(NsTag, Option<Vec<XmlAttribute>>), AnyError>` - The resolved namespaced tag
    ///   and the attribute list to create the element with.
    fn build_ns_child_tag(
        &self,
        parent_id: NodeId,
        ns_declaration: &NamespaceDeclaration,
        local_name: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<(NsTag, Option<Vec<XmlAttribute>>), AnyError> {
        let ns_context = self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_ns_context();
        let (alias, needs_declaration) = ns_declaration.resolve_in(&ns_context.borrow());
        let ns_tag = if alias.is_empty() {
            local_name.to_owned()
        } else {
            format!("{}:{}", alias, local_name)
        };
        if needs_declaration {
            let xmlns_name = if alias.is_empty() {
                "xmlns".to_owned()
            } else {
                format!("xmlns:{}", alias)
            };
            let mut augmented_attributes = attributes.unwrap_or_default();
            augmented_attributes.push(XmlAttribute::new(xmlns_name, ns_declaration.uri.to_owned()));
            Ok((ns_tag, Some(augmented_attributes)))
        } else {
            Ok((ns_tag, attributes))
        }
    }

    /// Creates the root element from a fully-formed `tag` in a fresh namespace scope.
    fn insert_root_element(
        &mut self,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        self.running_id += 1;
        let node_id = self.running_id;
        let mut element =
            XmlElement::new(tag, attributes, Rc::new(RefCell::new(XmlNamespace::new())))
                .context("draviavemal-xml_rs::Failed to create element")?;
        element.set_id_mut(node_id);
        self.add_element(node_id, element);
        debug!(
            "draviavemal-xml_rs::Created root element <{}> with node id {}",
            tag, node_id
        );
        Ok(node_id)
    }

    /// Creates a child from a fully-formed `tag` and appends it to the parent's contents.
    fn insert_child_element(
        &mut self,
        parent_id: NodeId,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (node_id, tag, tag_ns) =
            self.create_insert_element_into_collection(parent_id, tag, attributes)?;
        self.get_element_mut(parent_id)
            .context("draviavemal-xml_rs::Parent element not found")?
            .add_child_mut(node_id, &tag, &tag_ns)
            .context("draviavemal-xml_rs::Failed to add child element to parent")?;
        trace!(
            "draviavemal-xml_rs::Appended child element <{}> (node {}) to parent node {}",
            tag_ns,
            node_id,
            parent_id
        );
        Ok(node_id)
    }

    /// Creates a child from a fully-formed `tag` and inserts it after `reference`.
    fn insert_child_after(
        &mut self,
        parent_id: NodeId,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
        reference: String,
        match_by_ns: bool,
    ) -> Result<NodeId, AnyError> {
        let (node_id, tag, tag_ns) =
            self.create_insert_element_into_collection(parent_id, tag, attributes)?;
        let parent = self
            .get_element_mut(parent_id)
            .context("draviavemal-xml_rs::Parent element not found")?;
        if match_by_ns {
            parent
                .add_child_after_tag_ns_mut(node_id, &tag, &tag_ns, &reference)
                .context("draviavemal-xml_rs::Failed to add child element to parent")?;
        } else {
            parent
                .add_child_after_tag_mut(node_id, &tag, &tag_ns, &reference)
                .context("draviavemal-xml_rs::Failed to add child element to parent")?;
        }
        Ok(node_id)
    }

    /// Creates a child from a fully-formed `tag` and inserts it before `reference`.
    fn insert_child_before(
        &mut self,
        parent_id: NodeId,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
        reference: String,
        match_by_ns: bool,
    ) -> Result<NodeId, AnyError> {
        let (node_id, tag, tag_ns) =
            self.create_insert_element_into_collection(parent_id, tag, attributes)?;
        let parent = self
            .get_element_mut(parent_id)
            .context("draviavemal-xml_rs::Parent element not found")?;
        if match_by_ns {
            parent
                .add_child_before_tag_ns_mut(node_id, &tag, &tag_ns, &reference)
                .context("draviavemal-xml_rs::Failed to add child element to parent")?;
        } else {
            parent
                .add_child_before_tag_mut(node_id, &tag, &tag_ns, &reference)
                .context("draviavemal-xml_rs::Failed to add child element to parent")?;
        }
        Ok(node_id)
    }

    /// Resolves `local_name` + `ns_declaration` into the namespaced tag used to match a reference.
    fn build_reference_ns_tag(
        &self,
        parent_id: NodeId,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
    ) -> Result<String, AnyError> {
        let ns_context = self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_ns_context();
        let (alias, _) = ns_declaration.resolve_in(&ns_context.borrow());
        Ok(if alias.is_empty() {
            local_name.to_owned()
        } else {
            format!("{}:{}", alias, local_name)
        })
    }

    fn create_insert_element_into_collection(
        &mut self,
        parent_id: NodeId,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<(NodeId, Tag, NsTag), AnyError> {
        self.running_id += 1;
        let node_id = self.running_id;
        let ns_context = self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_ns_context();
        let mut child_element = XmlElement::new(tag, attributes, ns_context)
            .context("draviavemal-xml_rs::Failed to create child element")?;
        child_element.set_id_mut(node_id);
        child_element.set_parent_id_mut(parent_id);
        let tag = child_element.get_tag();
        let tag_ns = child_element.get_tag_ns();
        self.add_element(node_id, child_element);
        Ok((node_id, tag, tag_ns))
    }
}
