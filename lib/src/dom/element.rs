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
    /// Declares namespaces on this element's scope for later manual use.
    ///
    /// Only URIs not already in scope are added, so duplicates are ignored.
    pub fn add_namespaces_mut(&mut self, ns_declarations: &[NamespaceDeclaration]) {
        for ns_declaration in ns_declarations {
            let (alias, needs_declaration) =
                ns_declaration.resolve_in(&self.namespace_context.borrow());
            if needs_declaration {
                self.ensure_namespace_scope_mut(&alias, ns_declaration.uri);
            }
        }
    }

    /// Resolves the alias for `ns_declaration` in this element's scope, declaring it if missing.
    pub fn resolve_alias_mut(&mut self, ns_declaration: &NamespaceDeclaration) -> String {
        let (alias, needs_declaration) =
            ns_declaration.resolve_in(&self.namespace_context.borrow());
        if needs_declaration {
            self.ensure_namespace_scope_mut(&alias, ns_declaration.uri);
        }
        alias
    }

    /// Prefer [`XmlElement::add_attribute_ns_mut`] for round-trippable namespaces.
    ///
    /// Adds an attribute from a raw `name` and `value`; any prefix in `name` must already be
    /// declared in scope, and the resulting namespaced name must be unique on this element.
    pub fn add_attribute_mut(&mut self, name: &str, value: &str) -> AnyResult<(), AnyError> {
        self.insert_attribute_mut(XmlAttribute::new(name.to_owned(), value.to_owned()))
    }

    /// Adds a namespaced attribute whose alias is resolved from `ns_declaration`, declaring it
    /// in scope if missing. The resulting namespaced name must be unique on this element.
    pub fn add_attribute_ns_mut(
        &mut self,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
        value: &str,
    ) -> AnyResult<(), AnyError> {
        let attribute = self.build_ns_attribute(local_name, ns_declaration, value);
        self.insert_attribute_mut(attribute)
    }

    /// Prefer [`XmlElement::add_replace_attribute_ns_mut`] for round-trippable namespaces.
    ///
    /// Adds or replaces an attribute from a raw `name` and `value`, matched by namespaced name;
    /// any prefix in `name` must already be declared in scope.
    pub fn add_replace_attribute_mut(
        &mut self,
        name: &str,
        value: &str,
    ) -> AnyResult<(), AnyError> {
        self.replace_attribute_mut(XmlAttribute::new(name.to_owned(), value.to_owned()))
    }

    /// Adds or replaces a namespaced attribute whose alias is resolved from `ns_declaration`,
    /// declaring it in scope if missing. The attribute is matched by its namespaced name.
    pub fn add_replace_attribute_ns_mut(
        &mut self,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
        value: &str,
    ) -> AnyResult<(), AnyError> {
        let attribute = self.build_ns_attribute(local_name, ns_declaration, value);
        self.replace_attribute_mut(attribute)
    }

    /// Sets the initial attributes of this element from a vector.
    ///
    /// Prefer [`XmlElement::add_attribute_ns_mut`] to add namespaced attributes incrementally.
    /// Fails if the element already has attributes or a prefix is not declared in scope.
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

    /// Prefer [`XmlElement::remove_attribute_ns_mut`] for round-trippable namespaces.
    ///
    /// Removes an attribute matched by `name`; a prefixed `name` is matched by its namespaced
    /// name, otherwise every attribute sharing that local name is removed regardless of prefix.
    ///
    /// # Arguments
    /// * `name` - The attribute name to remove, optionally namespaced (e.g., "ns:attr").
    pub fn remove_attribute_mut(&mut self, name: &str) {
        let match_by_ns = name.contains(':');
        if let Some(attributes) = &mut self.attributes {
            let removed_aliases: Vec<String> = attributes
                .iter()
                .filter(|attribute| Self::attribute_name_matches(attribute, name, match_by_ns))
                .filter_map(|attribute| attribute.get_ns_alias().map(str::to_string))
                .collect();
            attributes
                .retain(|attribute| !Self::attribute_name_matches(attribute, name, match_by_ns));
            let mut namespace_context = self.namespace_context.borrow_mut();
            for alias in removed_aliases {
                namespace_context.decrement_alias_use_mut(&alias);
            }
        }
    }

    /// Removes an attribute whose alias is resolved from `ns_declaration` in this element's scope.
    ///
    /// # Arguments
    /// * `local_name` - The attribute local name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the attribute's namespace.
    pub fn remove_attribute_ns_mut(
        &mut self,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
    ) {
        let ns_name = self.build_scoped_ns_name(local_name, ns_declaration);
        self.remove_attribute_mut(&ns_name);
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

    /// Replaces every direct text node of this element with a single text node.
    ///
    /// Child elements and comments are left untouched; only text content is swapped.
    ///
    /// # Arguments
    /// * `text` - The text content that becomes the element's only text node.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Ok on success, or an error if the content could not be stored.
    pub fn set_text_mut(&mut self, text: &str) -> AnyResult<(), AnyError> {
        if let Some(contents) = &mut self.child_contents {
            contents.retain(|content| !matches!(content, XmlElementContentType::Text(_)));
        }
        self.add_child_content_mut(XmlElementContentType::Text(text.to_owned()))
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
    /// Retrieves an attribute by namespace declaration and local name.
    ///
    /// Matches by resolving each prefixed attribute's alias to its URI, so the lookup is
    /// independent of the alias actually used in the document.
    ///
    /// # Arguments
    /// * `local_name` - The attribute local name without prefix.
    /// * `ns_declaration` - The namespace declaration whose URI identifies the attribute.
    ///
    /// # Returns
    /// * `Option<&XmlAttribute>` - A reference to the attribute if found, or None.
    pub fn get_attribute_ns(
        &self,
        attr_name: &str,
        ns_declaration: &NamespaceDeclaration,
    ) -> Option<&XmlAttribute> {
        self.get_attribute_by_uri(ns_declaration.uri, attr_name)
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

    /// Read-only counterpart to [`XmlElement::resolve_alias_mut`]; never declares a missing binding.
    #[allow(dead_code)] // retained as internal API; currently exercised only by tests
    pub(crate) fn resolve_alias(&self, ns_declaration: &NamespaceDeclaration) -> String {
        ns_declaration
            .resolve_in(&self.namespace_context.borrow())
            .0
    }

    /// Prefer [`XmlElement::get_attribute_ns`] for round-trippable namespaces.
    ///
    /// Retrieves an attribute matched by `name`; a prefixed `name` is matched by its namespaced
    /// name, otherwise the first attribute with that local name is returned regardless of prefix.
    ///
    /// # Arguments
    /// * `name` - The attribute name to retrieve, optionally namespaced (e.g., "ns:attr").
    ///
    /// # Returns
    /// * `Option<&XmlAttribute>` - A reference to the attribute if found, or None.
    pub fn get_attribute(&self, name: &str) -> Option<&XmlAttribute> {
        let match_by_ns = name.contains(':');
        self.attributes
            .as_ref()?
            .iter()
            .find(|item| Self::attribute_name_matches(item, name, match_by_ns))
    }

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

    /// Prefer [`XmlElement::find_first_child_ns`] for round-trippable namespaces.
    ///
    /// Finds the first child element matching `tag`; a prefixed `tag` is matched by its
    /// namespaced name, otherwise by local name.
    ///
    /// # Arguments
    /// * `tag` - The tag name to search for, optionally namespaced (e.g., "ns:tag").
    ///
    /// # Returns
    /// * `Option<NodeId>` - The NodeId of the first matching child, or None if not found.
    pub fn find_first_child(&self, tag: &str) -> Option<NodeId> {
        let match_by_ns = tag.contains(':');
        self.child_contents
            .as_ref()?
            .iter()
            .find_map(|content| match content {
                XmlElementContentType::Element((child_id, child_tag, child_tag_ns))
                    if Self::child_tag_matches(child_tag, child_tag_ns, tag, match_by_ns) =>
                {
                    Some(*child_id)
                }
                _ => None,
            })
    }

    /// Finds the first child element whose alias is resolved from `ns_declaration` in this
    /// element's scope and whose local name matches `local_name`.
    ///
    /// # Arguments
    /// * `local_name` - The local tag name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the tag's namespace.
    ///
    /// # Returns
    /// * `Option<NodeId>` - The NodeId of the first matching child, or None if not found.
    pub fn find_first_child_ns(
        &self,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
    ) -> Option<NodeId> {
        let tag_ns = self.build_scoped_ns_name(local_name, ns_declaration);
        self.child_contents
            .as_ref()?
            .iter()
            .find_map(|content| match content {
                XmlElementContentType::Element((child_id, _, child_tag_ns))
                    if *child_tag_ns == tag_ns =>
                {
                    Some(*child_id)
                }
                _ => None,
            })
    }

    /// Prefer [`XmlElement::find_all_child_ns`] for round-trippable namespaces.
    ///
    /// Finds all child elements matching `tag`; a prefixed `tag` is matched by its namespaced
    /// name, otherwise by local name.
    ///
    /// # Arguments
    /// * `tag` - The tag name to search for, optionally namespaced (e.g., "ns:tag").
    ///
    /// # Returns
    /// * `Option<Vec<NodeId>>` - A vector of matching child NodeIds, or None if none found.
    pub fn find_all_child(&self, tag: &str) -> Option<Vec<NodeId>> {
        let match_by_ns = tag.contains(':');
        let childs: Vec<NodeId> = self
            .child_contents
            .as_ref()?
            .iter()
            .filter_map(|content| match content {
                XmlElementContentType::Element((child_id, child_tag, child_tag_ns))
                    if Self::child_tag_matches(child_tag, child_tag_ns, tag, match_by_ns) =>
                {
                    Some(*child_id)
                }
                _ => None,
            })
            .collect();

        if childs.is_empty() {
            None
        } else {
            Some(childs)
        }
    }

    /// Finds all child elements whose alias is resolved from `ns_declaration` in this element's
    /// scope and whose local name matches `local_name`.
    ///
    /// # Arguments
    /// * `local_name` - The local tag name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the tag's namespace.
    ///
    /// # Returns
    /// * `Option<Vec<NodeId>>` - A vector of matching child NodeIds, or None if none found.
    pub fn find_all_child_ns(
        &self,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
    ) -> Option<Vec<NodeId>> {
        let tag_ns = self.build_scoped_ns_name(local_name, ns_declaration);
        let childs: Vec<NodeId> = self
            .child_contents
            .as_ref()?
            .iter()
            .filter_map(|content| match content {
                XmlElementContentType::Element((child_id, _, child_tag_ns))
                    if *child_tag_ns == tag_ns =>
                {
                    Some(*child_id)
                }
                _ => None,
            })
            .collect();

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

    /// Gets the namespace prefix applied to this element's tag, if any.
    ///
    /// # Returns
    /// * `Option<String>` - The prefix (e.g., "ns"), or None when the tag is unprefixed.
    pub fn get_prefix(&self) -> Option<String> {
        self.ns_alias.clone().filter(|alias| !alias.is_empty())
    }

    /// Resolves the namespace URI bound to this element's tag in its own scope.
    ///
    /// # Returns
    /// * `Option<String>` - The namespace URI, or None when the tag is in no namespace.
    pub fn get_namespace_uri(&self) -> Option<String> {
        let alias = self.ns_alias.as_deref().unwrap_or("");
        self.resolve_alias_to_uri(alias)
    }

    /// Prefer [`XmlElement::has_attribute_ns`] for round-trippable namespaces.
    ///
    /// Reports whether an attribute matching `name` exists; a prefixed `name` is matched by its
    /// namespaced name, otherwise by local name.
    ///
    /// # Arguments
    /// * `name` - The attribute name to check, optionally namespaced (e.g., "ns:attr").
    ///
    /// # Returns
    /// * `bool` - True when a matching attribute exists.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.get_attribute(name).is_some()
    }

    /// Reports whether an attribute resolved from `ns_declaration` with local name `local_name`
    /// exists, matching by URI independently of the alias used.
    ///
    /// # Arguments
    /// * `attr_name` - The attribute local name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the attribute's namespace.
    ///
    /// # Returns
    /// * `bool` - True when a matching attribute exists.
    pub fn has_attribute_ns(&self, attr_name: &str, ns_declaration: &NamespaceDeclaration) -> bool {
        self.get_attribute_ns(attr_name, ns_declaration).is_some()
    }

    /// Prefer [`XmlElement::get_attribute_value_ns`] for round-trippable namespaces.
    ///
    /// Retrieves the value of the attribute matching `name`; a prefixed `name` is matched by its
    /// namespaced name, otherwise by local name.
    ///
    /// # Arguments
    /// * `name` - The attribute name to read, optionally namespaced (e.g., "ns:attr").
    ///
    /// # Returns
    /// * `Option<&str>` - The attribute value if found, or None.
    pub fn get_attribute_value(&self, name: &str) -> Option<&str> {
        self.get_attribute(name)
            .map(|attribute| attribute.get_value())
    }

    /// Retrieves the value of the attribute resolved from `ns_declaration` with local name
    /// `local_name`, matching by URI independently of the alias used.
    ///
    /// # Arguments
    /// * `attr_name` - The attribute local name without prefix.
    /// * `ns_declaration` - The namespace declaration identifying the attribute's namespace.
    ///
    /// # Returns
    /// * `Option<&str>` - The attribute value if found, or None.
    pub fn get_attribute_value_ns(
        &self,
        attr_name: &str,
        ns_declaration: &NamespaceDeclaration,
    ) -> Option<&str> {
        self.get_attribute_ns(attr_name, ns_declaration)
            .map(|attribute| attribute.get_value())
    }

    /// Concatenates the text of every direct text node of this element.
    ///
    /// # Returns
    /// * `String` - The joined text, empty when the element holds no direct text.
    pub fn get_text_content(&self) -> String {
        let mut text = String::new();
        if let Some(contents) = &self.child_contents {
            for content in contents {
                if let XmlElementContentType::Text(value) = content {
                    text.push_str(value);
                }
            }
        }
        text
    }

    /// Retrieves the node IDs of all direct child elements, in document order.
    ///
    /// # Returns
    /// * `Option<Vec<NodeId>>` - The child element IDs, or None when there are none.
    pub fn get_child_element_ids(&self) -> Option<Vec<NodeId>> {
        let ids: Vec<NodeId> = self
            .child_contents
            .as_ref()?
            .iter()
            .filter_map(|content| match content {
                XmlElementContentType::Element((child_id, _, _)) => Some(*child_id),
                _ => None,
            })
            .collect();
        if ids.is_empty() {
            None
        } else {
            Some(ids)
        }
    }

    /// Gets the node ID of the first direct child element.
    ///
    /// # Returns
    /// * `Option<NodeId>` - The first child element ID, or None when there are none.
    pub fn get_first_child_element(&self) -> Option<NodeId> {
        self.child_contents
            .as_ref()?
            .iter()
            .find_map(|content| match content {
                XmlElementContentType::Element((child_id, _, _)) => Some(*child_id),
                _ => None,
            })
    }

    /// Gets the node ID of the last direct child element.
    ///
    /// # Returns
    /// * `Option<NodeId>` - The last child element ID, or None when there are none.
    pub fn get_last_child_element(&self) -> Option<NodeId> {
        self.child_contents
            .as_ref()?
            .iter()
            .rev()
            .find_map(|content| match content {
                XmlElementContentType::Element((child_id, _, _)) => Some(*child_id),
                _ => None,
            })
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

    /// Resolves `local_name` + `ns_declaration` into the namespaced name used inside this scope.
    fn build_scoped_ns_name(
        &self,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
    ) -> String {
        let (alias, _) = ns_declaration.resolve_in(&self.namespace_context.borrow());
        if alias.is_empty() {
            local_name.to_owned()
        } else {
            format!("{}:{}", alias, local_name)
        }
    }

    /// Matches an attribute against `name`, by namespaced name when `match_by_ns`, else local name.
    fn attribute_name_matches(attribute: &XmlAttribute, name: &str, match_by_ns: bool) -> bool {
        if match_by_ns {
            attribute.get_ns_name() == name
        } else {
            attribute.get_name() == name
        }
    }

    /// Matches a child tag against `tag`, by namespaced name when `match_by_ns`, else local name.
    fn child_tag_matches(
        child_tag: &str,
        child_tag_ns: &str,
        tag: &str,
        match_by_ns: bool,
    ) -> bool {
        if match_by_ns {
            child_tag_ns == tag
        } else {
            child_tag == tag
        }
    }

    /// Builds a namespaced [`XmlAttribute`] whose alias is resolved from `ns_declaration`.
    ///
    /// When the URI is not yet in scope the binding is declared, promoting the element to its
    /// own namespace context.
    fn build_ns_attribute(
        &mut self,
        local_name: &str,
        ns_declaration: &NamespaceDeclaration,
        value: &str,
    ) -> XmlAttribute {
        let (alias, needs_declaration) =
            ns_declaration.resolve_in(&self.namespace_context.borrow());
        if needs_declaration {
            self.ensure_namespace_scope_mut(&alias, ns_declaration.uri);
        }
        let attribute_name = if alias.is_empty() {
            local_name.to_owned()
        } else {
            format!("{}:{}", alias, local_name)
        };
        XmlAttribute::new(attribute_name, value.to_owned())
    }

    /// Inserts a unique attribute, rejecting duplicates and undeclared aliases.
    fn insert_attribute_mut(&mut self, attribute: XmlAttribute) -> AnyResult<(), AnyError> {
        let attributes = self.attributes.get_or_insert_with(Vec::new);
        if !attribute.is_valid_ns_alias(&self.namespace_context.borrow()) {
            warn!(
                "draviavemal-xml_rs::Rejected attribute '{}': namespace alias not declared in scope",
                attribute.get_ns_name()
            );
            return Err(AnyError::msg(
                "draviavemal-xml_rs::Add attribute namespace alias used without refering schema",
            ));
        }
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

    /// Adds or replaces an attribute matched by its namespaced name.
    fn replace_attribute_mut(&mut self, attribute: XmlAttribute) -> AnyResult<(), AnyError> {
        let attributes = self.attributes.get_or_insert_with(Vec::new);
        if !attribute.is_valid_ns_alias(&self.namespace_context.borrow()) {
            warn!(
                "draviavemal-xml_rs::Rejected replace of attribute '{}': namespace alias not declared in scope",
                attribute.get_ns_name()
            );
            return Err(AnyError::msg(
                "draviavemal-xml_rs::Replace attribute namespace alias used without refering schema",
            ));
        }
        let existing_index = attributes.iter().position(|existing_attribute| {
            existing_attribute.get_ns_name() == attribute.get_ns_name()
        });
        match existing_index {
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

    /// Ensures this element owns a namespace scope binding `alias -> uri`.
    ///
    /// A shared parent scope is cloned into a private overriding context before the binding
    /// is added, so declaring a namespace here never leaks to sibling elements.
    pub(crate) fn ensure_namespace_scope_mut(&mut self, alias: &str, uri: &str) {
        if !self.ns_context_override {
            let inherited = (*self.namespace_context.borrow()).clone();
            self.namespace_context = Rc::new(RefCell::new(inherited));
            self.ns_context_override = true;
            self.namespace_context
                .borrow_mut()
                .clear_local_declarations_mut();
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

    /// Reports whether an attribute matches the given name and value.
    ///
    /// # Arguments
    /// * `attr_name` - The name of the attribute to check, optionally namespaced.
    /// * `attr_value` - The expected value of the attribute.
    ///
    /// # Returns
    /// * `bool` - True if the element has an attribute with the given name and value.
    pub(crate) fn attribute_value_matches(&self, attr_name: &str, attr_value: &str) -> bool {
        let match_by_ns = attr_name.contains(':');
        if let Some(attributes) = &self.attributes {
            attributes.iter().any(|a| {
                Self::attribute_name_matches(a, attr_name, match_by_ns)
                    && a.get_value() == attr_value
            })
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

    /// Returns the aliases declared directly on this element with their URIs, in declaration order.
    pub(crate) fn get_local_namespace_declarations(&self) -> Vec<(String, String)> {
        self.namespace_context.borrow().get_local_declarations()
    }

    /// Returns the namespace alias applied to this element's tag, if any.
    pub(crate) fn get_tag_alias(&self) -> Option<&str> {
        self.ns_alias.as_deref()
    }

    /// Resolves an alias to its URI using this element's in-scope bindings.
    pub(crate) fn resolve_alias_to_uri(&self, alias: &str) -> Option<String> {
        self.namespace_context
            .borrow()
            .get_url(alias)
            .map(|(uri, _)| uri.clone())
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
                    namespace_context
                        .borrow_mut()
                        .clear_local_declarations_mut();

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
