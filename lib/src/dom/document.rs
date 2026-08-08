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
    // =====================================================================
    //  RECOMMENDED — namespace-aware, mutating API (robust, preferred)
    // =====================================================================
    // Build elements from a `NamespaceDeclaration`: the alias is resolved from the live
    // scope and `xmlns` is emitted only when required. Prefer these to guarantee valid,
    // round-trippable namespace handling.

    /// Creates the root element from a namespace declaration.
    ///
    /// The root starts with an empty scope, so the declaration's alias (`alias_override`
    /// when set, otherwise `default_alias`) is always emitted as an `xmlns` declaration.
    ///
    /// # Arguments
    /// * `declaration` - The namespace declaration describing the URI and preferred alias.
    /// * `local_name` - The root local name without prefix.
    /// * `attributes` - Optional additional attributes for the root element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created root element, or an error if
    ///   the tag or an attribute name is invalid.
    pub fn create_root_element_ns_mut(
        &mut self,
        declaration: &NamespaceDeclaration,
        local_name: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let alias = declaration.alias_override.unwrap_or(declaration.default_alias);
        let (ns_tag, xmlns_name) = if alias.is_empty() {
            (local_name.to_owned(), "xmlns".to_owned())
        } else {
            (format!("{}:{}", alias, local_name), format!("xmlns:{}", alias))
        };
        let mut augmented_attributes = attributes.unwrap_or_default();
        augmented_attributes.push(XmlAttribute::new(xmlns_name, declaration.uri.to_owned()));
        self.create_root_element_mut(&ns_tag, Some(augmented_attributes))
    }

    /// Appends a child element resolved through a namespace declaration.
    ///
    /// The alias for `declaration.uri` is resolved against the parent scope; when the URI is
    /// not already declared an `xmlns` declaration is injected so the child introduces the
    /// binding, otherwise the existing in-scope alias is reused.
    ///
    /// # Arguments
    /// * `parent_id` - The node ID of the parent element.
    /// * `declaration` - The namespace declaration describing the URI and preferred alias.
    /// * `local_name` - The child local name without prefix.
    /// * `attributes` - Optional additional attributes for the child element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created child element, or an error.
    pub fn append_child_element_ns_mut(
        &mut self,
        parent_id: NodeId,
        declaration: &NamespaceDeclaration,
        local_name: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (ns_tag, attributes) =
            self.build_ns_child_tag(parent_id, declaration, local_name, attributes)?;
        self.append_child_element_mut(parent_id, &ns_tag, attributes)
    }

    /// Inserts a namespace-declared child after the last occurrence of a namespaced tag.
    ///
    /// Resolves the new element's tag through `declaration` (as with
    /// [`XmlDocument::append_child_element_ns_mut`]) while positioning it relative to the
    /// existing `last_tag_ns` reference. If the reference tag is absent the child is appended.
    ///
    /// # Arguments
    /// * `parent_id` - The node ID of the parent element.
    /// * `declaration` - The namespace declaration describing the URI and preferred alias.
    /// * `local_name` - The new child local name without prefix.
    /// * `last_tag_ns` - The namespaced tag to position after (reference point).
    /// * `attributes` - Optional additional attributes for the new child element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created child element, or an error.
    pub fn inser_child_element_after_last_tag_ns_decl_mut(
        &mut self,
        parent_id: NodeId,
        declaration: &NamespaceDeclaration,
        local_name: &str,
        last_tag_ns: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (ns_tag, attributes) =
            self.build_ns_child_tag(parent_id, declaration, local_name, attributes)?;
        self.inser_child_element_after_last_tag_ns_mut(parent_id, &ns_tag, last_tag_ns, attributes)
    }

    /// Inserts a namespace-declared child before the first occurrence of a namespaced tag.
    ///
    /// Resolves the new element's tag through `declaration` (as with
    /// [`XmlDocument::append_child_element_ns_mut`]) while positioning it relative to the
    /// existing `first_tag_ns` reference. If the reference tag is absent the child is inserted
    /// at the front.
    ///
    /// # Arguments
    /// * `parent_id` - The node ID of the parent element.
    /// * `declaration` - The namespace declaration describing the URI and preferred alias.
    /// * `local_name` - The new child local name without prefix.
    /// * `first_tag_ns` - The namespaced tag to position before (reference point).
    /// * `attributes` - Optional additional attributes for the new child element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created child element, or an error.
    pub fn inser_child_element_before_first_tag_ns_decl_mut(
        &mut self,
        parent_id: NodeId,
        declaration: &NamespaceDeclaration,
        local_name: &str,
        first_tag_ns: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (ns_tag, attributes) =
            self.build_ns_child_tag(parent_id, declaration, local_name, attributes)?;
        self.inser_child_element_before_first_tag_ns_mut(
            parent_id, &ns_tag, first_tag_ns, attributes,
        )
    }

    /// Resolves the alias for a declaration in an element's scope, declaring it if missing.
    ///
    /// Delegates to [`XmlElement::resolve_alias_mut`]: when the URI is not already bound the
    /// element is promoted to its own namespace scope and the binding is registered.
    ///
    /// # Arguments
    /// * `element_id` - The node ID whose scope drives resolution.
    /// * `declaration` - The namespace declaration describing the URI and preferred alias.
    ///
    /// # Returns
    /// * `Result<String, AnyError>` - The alias now in scope for the declaration's URI, or an
    ///   error if the element does not exist.
    pub fn resolve_alias_mut(
        &mut self,
        element_id: NodeId,
        declaration: &NamespaceDeclaration,
    ) -> Result<String, AnyError> {
        Ok(self
            .get_element_mut(element_id)
            .context("draviavemal-xml_rs::Element not found for alias resolution")?
            .resolve_alias_mut(declaration))
    }

    // =====================================================================
    //  DEVELOPER HACK — direct string tags (maximum flexibility)
    // =====================================================================
    // These take a raw `prefix:name` tag and require any prefix to be declared in scope
    // already (via an `xmlns` attribute). They offer full control over the emitted markup;
    // prefer the namespace-aware methods above for documents that must round-trip.

    /// Creates and inserts the root element into the document from a raw tag string.
    ///
    /// Any prefix in `tag` must be declared through an accompanying `xmlns` attribute in
    /// `attributes`, which is folded into the document's namespace tree.
    ///
    /// # Recommendation
    /// Prefer [`XmlDocument::create_root_element_ns_mut`], which derives the prefix and emits the
    /// matching `xmlns` declaration from a [`NamespaceDeclaration`].
    ///
    /// # Arguments
    /// * `tag` - The tag name for the root element (e.g., "ns:root").
    /// * `attributes` - Optional attributes, including any `xmlns` declarations.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created root element, or an error.
    pub fn create_root_element_mut(
        &mut self,
        tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        // Generate a new unique ID for the element
        self.running_id += 1;
        let node_id = self.running_id;

        // Create the element with a new namespace context
        let mut element =
            XmlElement::new(tag, attributes, Rc::new(RefCell::new(XmlNamespace::new())))
                .context("draviavemal-xml_rs::Failed to create element")?;

        // Set the element's ID
        element.set_id_mut(node_id);

        // Add the element to the collection
        self.add_element(node_id, element);

        debug!(
            "draviavemal-xml_rs::Created root element <{}> with node id {}",
            tag, node_id
        );
        Ok(node_id)
    }

    /// Adds a child element to a parent element from a raw tag string.
    ///
    /// Any prefix in `new_tag` must already be declared in the parent scope, or be introduced
    /// through an `xmlns` attribute in `attributes`.
    ///
    /// # Recommendation
    /// Prefer [`XmlDocument::append_child_element_ns_mut`], which resolves the prefix from the
    /// parent scope and declares it only when needed.
    ///
    /// # Arguments
    /// * `parent_id` - The node ID of the parent element.
    /// * `new_tag` - The tag name for the child element (e.g., "ns:child").
    /// * `attributes` - Optional attributes for the child element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created child element, or an error.
    pub fn append_child_element_mut(
        &mut self,
        parent_id: NodeId,
        new_tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (node_id, tag, tag_ns) =
            self.create_insert_element_into_collection(parent_id, new_tag, attributes)?;

        // Add the child to the parent's contents
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

    /// Inserts a child element after the last occurrence of a specific local tag.
    ///
    /// Finds the last child with the given local tag name and inserts the new element
    /// immediately after it; if no match is found the new element is appended at the end.
    ///
    /// # Recommendation
    /// Prefer [`XmlDocument::inser_child_element_after_last_tag_ns_decl_mut`] for namespaced tags,
    /// which resolves the new element's prefix from scope.
    ///
    /// # Arguments
    /// * `parent_id` - The node ID of the parent element.
    /// * `new_tag` - The tag name for the new child element.
    /// * `last_tag` - The local tag name to position after (reference point).
    /// * `attributes` - Optional attributes for the new child element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created child element, or an error.
    pub fn inser_child_element_after_last_tag_mut(
        &mut self,
        parent_id: NodeId,
        new_tag: &str,
        last_tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (node_id, tag, tag_ns) =
            self.create_insert_element_into_collection(parent_id, new_tag, attributes)?;

        // Add the child to the parent's contents
        self.get_element_mut(parent_id)
            .context("draviavemal-xml_rs::Parent element not found")?
            .add_child_after_tag_mut(node_id, &tag, &tag_ns, last_tag)
            .context("draviavemal-xml_rs::Failed to add child element to parent")?;

        Ok(node_id)
    }

    /// Inserts a child element before the first occurrence of a specific local tag.
    ///
    /// Finds the first child with the given local tag name and inserts the new element
    /// immediately before it; if no match is found the new element is inserted at the front.
    ///
    /// # Recommendation
    /// Prefer [`XmlDocument::inser_child_element_before_first_tag_ns_decl_mut`] for namespaced
    /// tags, which resolves the new element's prefix from scope.
    ///
    /// # Arguments
    /// * `parent_id` - The node ID of the parent element.
    /// * `new_tag` - The tag name for the new child element.
    /// * `first_tag` - The local tag name to position before (reference point).
    /// * `attributes` - Optional attributes for the new child element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created child element, or an error.
    pub fn inser_child_element_before_first_tag_mut(
        &mut self,
        parent_id: NodeId,
        new_tag: &str,
        first_tag: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (node_id, tag, tag_ns) =
            self.create_insert_element_into_collection(parent_id, new_tag, attributes)?;

        // Add the child to the parent's contents
        self.get_element_mut(parent_id)
            .context("draviavemal-xml_rs::Parent element not found")?
            .add_child_before_tag_mut(node_id, &tag, &tag_ns, first_tag)
            .context("draviavemal-xml_rs::Failed to add child element to parent")?;

        Ok(node_id)
    }

    /// Inserts a child element after the last occurrence of a specific namespaced tag.
    ///
    /// Like [`XmlDocument::inser_child_element_after_last_tag_mut`] but matches the reference
    /// using the full namespaced tag name (e.g., "ns:tag") rather than just the local name.
    ///
    /// # Recommendation
    /// Prefer [`XmlDocument::inser_child_element_after_last_tag_ns_decl_mut`], which additionally
    /// resolves the new element's own prefix from a [`NamespaceDeclaration`].
    ///
    /// # Arguments
    /// * `parent_id` - The node ID of the parent element.
    /// * `new_tag` - The tag name for the new child element.
    /// * `last_tag_ns` - The namespaced tag to position after (reference point).
    /// * `attributes` - Optional attributes for the new child element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created child element, or an error.
    pub fn inser_child_element_after_last_tag_ns_mut(
        &mut self,
        parent_id: NodeId,
        new_tag: &str,
        last_tag_ns: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (node_id, tag, tag_ns) =
            self.create_insert_element_into_collection(parent_id, new_tag, attributes)?;

        // Add the child to the parent's contents
        self.get_element_mut(parent_id)
            .context("draviavemal-xml_rs::Parent element not found")?
            .add_child_after_tag_ns_mut(node_id, &tag, &tag_ns, last_tag_ns)
            .context("draviavemal-xml_rs::Failed to add child element to parent")?;

        Ok(node_id)
    }

    /// Inserts a child element before the first occurrence of a specific namespaced tag.
    ///
    /// Like [`XmlDocument::inser_child_element_before_first_tag_mut`] but matches the reference
    /// using the full namespaced tag name (e.g., "ns:tag") rather than just the local name.
    ///
    /// # Recommendation
    /// Prefer [`XmlDocument::inser_child_element_before_first_tag_ns_decl_mut`], which additionally
    /// resolves the new element's own prefix from a [`NamespaceDeclaration`].
    ///
    /// # Arguments
    /// * `parent_id` - The node ID of the parent element.
    /// * `new_tag` - The tag name for the new child element.
    /// * `first_tag_ns` - The namespaced tag to position before (reference point).
    /// * `attributes` - Optional attributes for the new child element.
    ///
    /// # Returns
    /// * `Result<NodeId, AnyError>` - The node ID of the created child element, or an error.
    pub fn inser_child_element_before_first_tag_ns_mut(
        &mut self,
        parent_id: NodeId,
        new_tag: &str,
        first_tag_ns: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<NodeId, AnyError> {
        let (node_id, tag, tag_ns) =
            self.create_insert_element_into_collection(parent_id, new_tag, attributes)?;

        // Add the child to the parent's contents
        self.get_element_mut(parent_id)
            .context("draviavemal-xml_rs::Parent element not found")?
            .add_child_before_tag_ns_mut(node_id, &tag, &tag_ns, first_tag_ns)
            .context("draviavemal-xml_rs::Failed to add child element to parent")?;

        Ok(node_id)
    }

    // =====================================================================
    //  SHARED — mutating lifecycle (namespace independent)
    // =====================================================================

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
    // =====================================================================
    //  RECOMMENDED — namespace-aware, read API (robust, preferred)
    // =====================================================================

    /// Resolves the alias a declaration would use in an element's scope without mutating it.
    ///
    /// Read-only counterpart to [`XmlDocument::resolve_alias_mut`]: applies the precedence
    /// `alias_override` > alias already bound to the URI > `default_alias` and never declares a
    /// missing binding.
    ///
    /// # Arguments
    /// * `element_id` - The node ID whose scope drives resolution.
    /// * `declaration` - The namespace declaration describing the URI and preferred alias.
    ///
    /// # Returns
    /// * `Result<String, AnyError>` - The alias resolution selects for the declaration's URI,
    ///   or an error if the element does not exist.
    pub fn resolve_alias(
        &self,
        element_id: NodeId,
        declaration: &NamespaceDeclaration,
    ) -> Result<String, AnyError> {
        Ok(self
            .get_element(element_id)
            .context("draviavemal-xml_rs::Element not found for alias resolution")?
            .resolve_alias(declaration))
    }

    // =====================================================================
    //  SHARED — neutral read / query / lifecycle
    // =====================================================================

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
            running_id: self.running_id,
            root_id: self.root_id,
            // Clone each element in the collection
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
            .context("draviavemal-xml_rs::Failed to pull parent element")?
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
            .context("draviavemal-xml_rs::Failed to pull parent element")?
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
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .find_all_child(tag))
    }

    /// Finds all child elements with the given namespaced tag name.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent element.
    /// * `tag_ns` - The namespaced tag to search for (e.g., "ns:tag").
    ///
    /// # Returns
    /// * `Result<Option<Vec<NodeId>, AnyError>` - A vector of matching child IDs, or None if none found.
    pub fn find_all_child_ns(
        &self,
        parent_id: NodeId,
        tag_ns: &str,
    ) -> Result<Option<Vec<NodeId>>, AnyError> {
        Ok(self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
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
                        .has_attribute(attr_name, attr_value)
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
                        .has_attribute_ns(attr_name_ns, attr_value)
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
        declaration: &NamespaceDeclaration,
        local_name: &str,
        attributes: Option<Vec<XmlAttribute>>,
    ) -> Result<(NsTag, Option<Vec<XmlAttribute>>), AnyError> {
        let ns_context = self
            .get_element(parent_id)
            .context("draviavemal-xml_rs::Failed to pull parent element")?
            .get_ns_context();
        let (alias, needs_declaration) = declaration.resolve_in(&ns_context.borrow());
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
            augmented_attributes.push(XmlAttribute::new(xmlns_name, declaration.uri.to_owned()));
            Ok((ns_tag, Some(augmented_attributes)))
        } else {
            Ok((ns_tag, attributes))
        }
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
