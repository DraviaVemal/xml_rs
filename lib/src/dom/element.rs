/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{
    utils::validation::is_valid_xml_name, NamespaceDeclaration, NodeId, XmlAttribute, XmlNamespace,
};
use anyhow::{Context, Error as AnyError, Result as AnyResult};
use log::{debug, trace, warn};
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
    // =====================================================================
    //  RECOMMENDED — namespace-aware, mutating API (robust, preferred)
    // =====================================================================
    // These resolve prefixes from the live document scope, keeping alias mapping and
    // `xmlns` emission correct. Prefer them over the raw-string helpers further down.

    /// Adds an attribute resolved through a namespace declaration.
    ///
    /// The alias for `declaration.uri` is resolved against this element's scope; when the
    /// URI is not yet declared the scope is promoted to its own namespace context and the
    /// binding is added so the serializer emits the matching `xmlns` declaration. When the
    /// URI is already in scope the existing alias is reused and no redundant declaration is
    /// produced.
    ///
    /// # Arguments
    /// * `declaration` - The namespace declaration describing the URI and preferred alias.
    /// * `local_name` - The attribute local name without prefix (e.g., "embed").
    /// * `value` - The attribute value.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Ok on success, or an error if an attribute with the same
    ///   resolved namespaced name already exists on this element.
    pub fn add_attribute_ns_mut(
        &mut self,
        declaration: &NamespaceDeclaration,
        local_name: &str,
        value: &str,
    ) -> AnyResult<(), AnyError> {
        let (alias, needs_declaration) =
            declaration.resolve_in(&self.namespace_context.borrow());
        if needs_declaration {
            self.ensure_namespace_scope_mut(&alias, declaration.uri);
        }
        let attribute_name = if alias.is_empty() {
            local_name.to_owned()
        } else {
            format!("{}:{}", alias, local_name)
        };
        self.add_attribute_mut(XmlAttribute::new(attribute_name, value.to_owned()))
    }

    /// Resolves the alias for a namespace declaration, declaring it in scope if missing.
    ///
    /// Unlike [`XmlElement::resolve_alias`], when the URI is not already bound this promotes
    /// the element to its own namespace context and registers the binding so the alias is
    /// valid for subsequent tag or attribute construction.
    ///
    /// # Arguments
    /// * `declaration` - The namespace declaration describing the URI and preferred alias.
    ///
    /// # Returns
    /// * `String` - The alias now in scope for the declaration's URI.
    pub fn resolve_alias_mut(&mut self, declaration: &NamespaceDeclaration) -> String {
        let (alias, needs_declaration) =
            declaration.resolve_in(&self.namespace_context.borrow());
        if needs_declaration {
            self.ensure_namespace_scope_mut(&alias, declaration.uri);
        }
        alias
    }

    // =====================================================================
    //  DEVELOPER HACK — direct string attributes (maximum flexibility)
    // =====================================================================
    // These trust the caller-supplied prefix verbatim and validate it only against the
    // aliases already declared in scope. They give full control over the emitted prefix;
    // prefer the namespace-aware methods above for documents that must round-trip cleanly.

    /// Adds an attribute to this element from a pre-built [`XmlAttribute`].
    ///
    /// The attribute name must be unique within the element. If an attribute with the
    /// same namespaced name already exists, an error is returned. Any prefix on the
    /// attribute must already be declared in scope or the call is rejected.
    ///
    /// # Recommendation
    /// Prefer [`XmlElement::add_attribute_ns_mut`], which resolves and, if necessary, declares
    /// the alias from the live scope instead of trusting a hard-coded prefix.
    ///
    /// # Arguments
    /// * `attribute` - The XML attribute to add to this element.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - An empty result if the attribute was added successfully,
    ///   or an error if an attribute with the same name already exists or uses an undeclared
    ///   namespace alias.
    pub fn add_attribute_mut(&mut self, attribute: XmlAttribute) -> AnyResult<(), AnyError> {
        let attributes = self.attributes.get_or_insert_with(Vec::new);
        // Validate ns alias if exist
        if !attribute.is_valid_ns_alias(&self.namespace_context.borrow()) {
            warn!(
                "draviavemal-xml_rs::Rejected attribute '{}': namespace alias not declared in scope",
                attribute.get_ns_name()
            );
            return Err(AnyError::msg(
                "draviavemal-xml_rs::Add attribute namespace alias used without refering schema",
            ));
        }
        // Reject duplicate attribute names to keep them unique per element
        if attributes
            .iter()
            .any(|existing_attribute| existing_attribute.get_ns_name() == attribute.get_ns_name())
        {
            warn!(
                "draviavemal-xml_rs::Rejected duplicate attribute '{}' on element node {}",
                attribute.get_ns_name(),
                self.id
            );
            return Err(AnyError::msg(format!(
                "draviavemal-xml_rs::Attribute '{}' already exists on this element",
                attribute.get_ns_name()
            )));
        }
        // Add the attribute to the attributes collection
        trace!(
            "draviavemal-xml_rs::Added attribute '{}' to element node {}",
            attribute.get_ns_name(),
            self.id
        );
        let attribute_alias = attribute.get_ns_alias().map(str::to_string);
        attributes.push(attribute);
        if let Some(alias) = attribute_alias {
            self.namespace_context
                .borrow_mut()
                .increment_alias_use_mut(&alias);
        }
        Ok(())
    }

    /// Adds or replaces an attribute on this element from a pre-built [`XmlAttribute`].
    ///
    /// The attribute is matched by its namespaced name. If an attribute with the same
    /// name already exists, it is replaced in-place at its current position. Otherwise,
    /// the attribute is appended to the end of the attribute list. Any prefix must already
    /// be declared in scope.
    ///
    /// # Recommendation
    /// For namespaced attributes prefer [`XmlElement::add_attribute_ns_mut`], which resolves the
    /// alias from scope; combine with [`XmlElement::remove_attribute_ns_mut`] when a true
    /// replace is required.
    ///
    /// # Arguments
    /// * `attribute` - The XML attribute to add or replace.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Ok on success, or an error if the attribute uses an
    ///   undeclared namespace alias.
    pub fn add_replace_attribute_mut(
        &mut self,
        attribute: XmlAttribute,
    ) -> AnyResult<(), AnyError> {
        let attributes = self.attributes.get_or_insert_with(Vec::new);
        // Validate ns alias if exist
        if !attribute.is_valid_ns_alias(&self.namespace_context.borrow()) {
            warn!(
                "draviavemal-xml_rs::Rejected replace of attribute '{}': namespace alias not declared in scope",
                attribute.get_ns_name()
            );
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
            Some(index) => {
                trace!(
                    "draviavemal-xml_rs::Replaced attribute '{}' on element node {}",
                    attribute.get_ns_name(),
                    self.id
                );
                let previous_alias = attributes[index].get_ns_alias().map(str::to_string);
                let replacement_alias = attribute.get_ns_alias().map(str::to_string);
                attributes[index] = attribute;
                let mut namespace_context = self.namespace_context.borrow_mut();
                if let Some(alias) = previous_alias {
                    namespace_context.decrement_alias_use_mut(&alias);
                }
                if let Some(alias) = replacement_alias {
                    namespace_context.increment_alias_use_mut(&alias);
                }
            }
            // Otherwise append to the end
            None => {
                trace!(
                    "draviavemal-xml_rs::Added attribute '{}' to element node {}",
                    attribute.get_ns_name(),
                    self.id
                );
                let attribute_alias = attribute.get_ns_alias().map(str::to_string);
                attributes.push(attribute);
                if let Some(alias) = attribute_alias {
                    self.namespace_context
                        .borrow_mut()
                        .increment_alias_use_mut(&alias);
                }
            }
        }
        Ok(())
    }

    /// Sets the initial attributes of this element from a vector.
    ///
    /// This is intended for initialising an element's attributes. If the element already
    /// has one or more attributes, an error is returned. Every prefixed attribute must use
    /// an alias already declared in scope.
    ///
    /// # Recommendation
    /// For adding namespaced attributes incrementally after creation prefer
    /// [`XmlElement::add_attribute_ns_mut`], which declares missing aliases automatically.
    ///
    /// # Arguments
    /// * `attributes` - The attributes to set on this element.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Ok on success, or an error if the element already
    ///   has attributes or an attribute uses an undeclared namespace alias.
    pub fn set_attribute_mut(&mut self, attributes: Vec<XmlAttribute>) -> AnyResult<(), AnyError> {
        // Only allow setting when there are no existing attributes
        if self.attributes.is_some() {
            warn!(
                "draviavemal-xml_rs::Rejected set_attribute on element node {}: attributes already present",
                self.id
            );
            return Err(AnyError::msg(
                "draviavemal-xml_rs::Element already has attributes; cannot set initial attributes",
            ));
        }
        // Validate attribute NS
        let attributes_valid = {
            let namespace = self.namespace_context.borrow();
            attributes
                .iter()
                .all(|attribute| attribute.is_valid_ns_alias(&namespace))
        };
        if !attributes_valid {
            warn!(
                "draviavemal-xml_rs::Rejected set_attribute on element node {}: an attribute uses an undeclared namespace alias",
                self.id
            );
            return Err(AnyError::msg(
                "draviavemal-xml_rs::Set attribute namespace alias used without refering schema",
            ));
        }
        {
            let mut namespace_context = self.namespace_context.borrow_mut();
            for attribute in &attributes {
                if let Some(alias) = attribute.get_ns_alias() {
                    namespace_context.increment_alias_use_mut(alias);
                }
            }
        }
        self.attributes = Some(attributes);
        Ok(())
    }

    /// Removes an attribute by its local name.
    ///
    /// Caution: This method does not consider namespaces. If multiple attributes share the
    /// same local name but different namespaces, all of them are removed.
    ///
    /// # Arguments
    /// * `name` - The local name of the attribute to remove.
    pub fn remove_attribute_mut(&mut self, name: &str) {
        if let Some(attributes) = &mut self.attributes {
            let removed_aliases: Vec<String> = attributes
                .iter()
                .filter(|attribute| attribute.get_name() == name)
                .filter_map(|attribute| attribute.get_ns_alias().map(str::to_string))
                .collect();
            attributes.retain(|attribute| attribute.get_name() != name);
            let mut namespace_context = self.namespace_context.borrow_mut();
            for alias in removed_aliases {
                namespace_context.decrement_alias_use_mut(&alias);
            }
        }
    }

    /// Removes an attribute by its namespaced name.
    ///
    /// # Arguments
    /// * `ns_name` - The namespaced name of the attribute to remove (e.g., "ns:attr").
    pub fn remove_attribute_ns_mut(&mut self, ns_name: &str) {
        if let Some(attributes) = &mut self.attributes {
            let removed_aliases: Vec<String> = attributes
                .iter()
                .filter(|attribute| attribute.get_ns_name() == ns_name)
                .filter_map(|attribute| attribute.get_ns_alias().map(str::to_string))
                .collect();
            attributes.retain(|attribute| attribute.get_ns_name() != ns_name);
            let mut namespace_context = self.namespace_context.borrow_mut();
            for alias in removed_aliases {
                namespace_context.decrement_alias_use_mut(&alias);
            }
        }
    }

    /// Clears all attributes of this element.
    ///
    /// # Returns
    /// * `AnyResult<u32, AnyError>` - The number of attributes that were removed.
    pub fn clear_attribute_mut(&mut self) -> AnyResult<u32, AnyError> {
        let removed_count = self.attributes.iter().flatten().count() as u32;
        if let Some(attributes) = &self.attributes {
            let mut namespace_context = self.namespace_context.borrow_mut();
            for attribute in attributes {
                if let Some(alias) = attribute.get_ns_alias() {
                    namespace_context.decrement_alias_use_mut(alias);
                }
            }
        }
        self.attributes = None;
        Ok(removed_count)
    }

    // =====================================================================
    //  SHARED — content mutation (namespace independent)
    // =====================================================================

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
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Ok on success, or an error if the content could not be stored.
    pub fn add_text_mut(&mut self, text: &str) -> AnyResult<(), AnyError> {
        self.add_child_content_mut(XmlElementContentType::Text(text.to_owned()))?;
        Ok(())
    }

    /// Adds a comment node to this element's contents.
    ///
    /// This method inserts XML comments (`<!-- comment -->`) into the element. Comments are
    /// preserved during serialization and can be used for documentation or to temporarily
    /// disable parts of the XML.
    ///
    /// # Arguments
    /// * `comment` - The comment text to add (without the `<!--` and `-->` delimiters).
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Ok on success, or an error if the content could not be stored.
    pub fn add_comments_mut(&mut self, comment: &str) -> AnyResult<(), AnyError> {
        self.add_child_content_mut(XmlElementContentType::Comment(comment.to_owned()))?;
        Ok(())
    }
}

impl XmlElement {
    // =====================================================================
    //  RECOMMENDED — namespace-aware, read API (robust, preferred)
    // =====================================================================
    // Look up attributes and aliases by namespace URI, so results are independent of the
    // prefix a document happens to use. Prefer these over the raw-name getters below.

    /// Retrieves an attribute by namespace declaration and local name.
    ///
    /// Matches by resolving each prefixed attribute's alias to its URI, so the lookup is
    /// independent of the alias actually used in the document.
    ///
    /// # Arguments
    /// * `declaration` - The namespace declaration whose URI identifies the attribute.
    /// * `local_name` - The attribute local name without prefix.
    ///
    /// # Returns
    /// * `Option<&XmlAttribute>` - A reference to the attribute if found, or None.
    pub fn get_attribute_by_ns(
        &self,
        declaration: &NamespaceDeclaration,
        local_name: &str,
    ) -> Option<&XmlAttribute> {
        self.get_attribute_by_uri(declaration.uri, local_name)
    }

    /// Retrieves an attribute by its namespace URI and local name.
    ///
    /// Resolves each prefixed attribute's alias against the element's namespace context
    /// and compares the resolved URI, making this alias-independent.
    ///
    /// # Arguments
    /// * `uri` - The namespace URI the attribute must resolve to.
    /// * `local_name` - The attribute local name without prefix.
    ///
    /// # Returns
    /// * `Option<&XmlAttribute>` - A reference to the attribute if found, or None.
    pub fn get_attribute_by_uri(&self, uri: &str, local_name: &str) -> Option<&XmlAttribute> {
        let ctx = self.namespace_context.borrow();
        self.attributes.as_ref()?.iter().find(|attr| {
            attr.get_name() == local_name
                && attr
                    .get_ns_alias()
                    .and_then(|alias| ctx.get_url(alias))
                    .map(|(resolved, _)| resolved == uri)
                    .unwrap_or(false)
        })
    }

    /// Returns the namespace alias currently in scope for the given URI, if any.
    ///
    /// # Arguments
    /// * `uri` - The namespace URI to look up.
    ///
    /// # Returns
    /// * `Option<String>` - The in-scope alias bound to `uri`, or None when it is not declared.
    pub fn get_alias_for_uri(&self, uri: &str) -> Option<String> {
        self.namespace_context.borrow().get_alias(uri).cloned()
    }

    /// Resolves the alias a namespace declaration would use in this element's scope.
    ///
    /// Read-only counterpart to [`XmlElement::resolve_alias_mut`]: applies the precedence
    /// `alias_override` > alias already bound to the URI > `default_alias` without mutating
    /// the namespace context, so it never declares a missing binding.
    ///
    /// # Arguments
    /// * `declaration` - The namespace declaration describing the URI and preferred alias.
    ///
    /// # Returns
    /// * `String` - The alias that resolution selects for the declaration's URI.
    pub fn resolve_alias(&self, declaration: &NamespaceDeclaration) -> String {
        declaration.resolve_in(&self.namespace_context.borrow()).0
    }

    // =====================================================================
    //  DEVELOPER HACK — direct name lookups (maximum flexibility)
    // =====================================================================
    // These match on the raw name/prefix and are alias-sensitive. Handy when the exact
    // prefix is known, but brittle across documents that use a different alias for the URI.

    /// Retrieves an attribute by its local name, ignoring namespaces.
    ///
    /// The first attribute whose local name matches is returned regardless of its prefix.
    ///
    /// # Recommendation
    /// For namespaced attributes prefer [`XmlElement::get_attribute_by_ns`], which matches by
    /// URI and is unaffected by which alias the document uses.
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

    /// Retrieves an attribute by its exact namespaced name.
    ///
    /// Matches the stored `prefix:local` string literally, so it only finds the attribute
    /// when the document uses the same alias.
    ///
    /// # Recommendation
    /// Prefer [`XmlElement::get_attribute_by_ns`], which matches by URI and tolerates any alias.
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

    // =====================================================================
    //  SHARED — neutral read / query (namespace independent)
    // =====================================================================

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

    /// Gets the tag name of this element without its namespace prefix.
    ///
    /// # Returns
    /// * `String` - The local tag name without any namespace prefix.
    pub fn get_tag(&self) -> String {
        self.tag.clone()
    }

    /// Gets the tag name of this element with its namespace alias if present.
    ///
    /// # Returns
    /// * `String` - The namespaced tag name (e.g., "ns:tag") or just the tag if no namespace.
    pub fn get_tag_ns(&self) -> String {
        match &self.ns_alias {
            Some(ns_alias) if !ns_alias.is_empty() => format!("{}:{}", ns_alias, self.tag),
            _ => self.tag.clone(),
        }
    }

    /// Retrieves all attribute local names, without namespace prefixes.
    ///
    /// # Returns
    /// * `Option<Vec<String>>` - The local names, or None if the element has no attributes.
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

    /// Retrieves all attribute names including their namespace prefixes.
    ///
    /// # Returns
    /// * `Option<Vec<String>>` - The namespaced names, or None if the element has no attributes.
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

    /// Gets the count of child elements, excluding text and comment nodes.
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

    /// Finds the first child element with the given local tag name.
    ///
    /// # Arguments
    /// * `tag` - The local tag name to search for.
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

    /// Finds the first child element with the given namespaced tag name.
    ///
    /// # Arguments
    /// * `tag_ns` - The namespaced tag name to search for (e.g., "ns:tag").
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

    /// Finds all child elements with the given local tag name.
    ///
    /// # Arguments
    /// * `tag` - The local tag name to search for.
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

    /// Finds all child elements with the given namespaced tag name.
    ///
    /// # Arguments
    /// * `tag_ns` - The namespaced tag name to search for (e.g., "ns:tag").
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

    /// Gets the text value of the element if present, else None.
    ///
    /// # Returns
    /// * `AnyResult<Option<String>, AnyError>` - The first text node's value, or None when the
    ///   element has no text content.
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

    /// Ensures this element owns a namespace scope binding `alias -> uri`.
    ///
    /// A shared parent scope is cloned into a private overriding context before the binding
    /// is added, so declaring a namespace here never leaks to sibling elements.
    pub(crate) fn ensure_namespace_scope_mut(&mut self, alias: &str, uri: &str) {
        if !self.ns_context_override {
            let inherited = (*self.namespace_context.borrow()).clone();
            self.namespace_context = Rc::new(RefCell::new(inherited));
            self.ns_context_override = true;
        }
        self.namespace_context
            .borrow_mut()
            .add_url_alias_mut(alias, uri);
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

    /// Releases this element's namespace usage counts from its scope when detached.
    pub(crate) fn release_ns_usage(&self) {
        let mut namespace_context = self.namespace_context.borrow_mut();
        match &self.ns_alias {
            Some(alias) => namespace_context.decrement_alias_use_mut(alias),
            None => namespace_context.decrement_alias_use_mut(""),
        }
        if let Some(attributes) = &self.attributes {
            for attribute in attributes {
                if let Some(alias) = attribute.get_ns_alias() {
                    namespace_context.decrement_alias_use_mut(alias);
                }
            }
        }
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
                    warn!("draviavemal-xml_rs::Rejected element <{}>: one or more attribute names are invalid XML names", new_tag);
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
                    debug!(
                        "draviavemal-xml_rs::Element <{}> declares {} namespace(s); creating overriding namespace scope",
                        new_tag,
                        namespaces.len()
                    );
                    let inherited = (*namespace_context.borrow()).clone();
                    namespace_context = Rc::new(RefCell::new(inherited));
                    namespace_context.borrow_mut().reset_alias_use_mut();

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
                    warn!(
                        "draviavemal-xml_rs::Rejected element <{}:{}>: namespace alias '{}' used without a referring schema",
                        ns, &tag[1..], ns
                    );
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
            let attributes_valid = match filtered_attributes.as_ref() {
                Some(attributes) => {
                    let namespace = namespace_context.borrow();
                    attributes
                        .iter()
                        .all(|attribute| attribute.is_valid_ns_alias(&namespace))
                }
                None => true,
            };
            if !attributes_valid {
                warn!("draviavemal-xml_rs::Rejected element <{}>: an attribute uses an undeclared namespace alias", new_tag);
                return Err(AnyError::msg(
                    "draviavemal-xml_rs::Attribute in new tag namespace alias used without refering schema",
                ));
            }

            {
                let mut namespace = namespace_context.borrow_mut();
                match &ns_alias {
                    Some(alias) => namespace.increment_alias_use_mut(alias),
                    None => namespace.increment_alias_use_mut(""),
                }
                if let Some(attributes) = filtered_attributes.as_ref() {
                    for attribute in attributes {
                        if let Some(alias) = attribute.get_ns_alias() {
                            namespace.increment_alias_use_mut(alias);
                        }
                    }
                }
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
            warn!(
                "draviavemal-xml_rs::Rejected element: '{}' is not a valid XML tag name",
                new_tag
            );
            Err(AnyError::msg(
                "draviavemal-xml_rs::draviavemal-xml_rs::Invalid XML tag name",
            ))
        }
    }
}
