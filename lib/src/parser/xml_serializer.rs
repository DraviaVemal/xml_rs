/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use super::namespace_optimizer::NamespaceOptimizer;
use crate::{log_elapsed, NodeId, XmlDocument, XmlElement, XmlElementContentType};
use anyhow::{Context, Error as AnyError, Result as AnyResult};
use log::{debug, error, info, trace};
use quick_xml::escape::escape;
use std::collections::HashMap;
use std::fs;

/// Emission plan mapping a node to the prefixed namespace declarations it must emit.
pub(crate) type NamespacePlan = HashMap<NodeId, Vec<(String, String)>>;

/// Options controlling how an [`XmlDocument`] is serialized.
#[derive(Debug, Clone)]
pub struct SerializeOptions {
    /// Hoists every namespace declaration to the lowest common ancestor of its uses and
    /// drops declarations that are never referenced.
    pub optimize_namespaces: bool,
}

impl Default for SerializeOptions {
    fn default() -> Self {
        Self {
            optimize_namespaces: false,
        }
    }
}

/// Provides XML serialization utilities for converting `XmlDocument` objects to XML text.
///
/// This struct contains methods to serialize an XML document structure into a string or file.
pub struct XmlSerializer {}

impl XmlSerializer {
    // --------------------------
    // pub methods
    // --------------------------

    /// Serializes an XML document tree to a file.
    ///
    /// # Arguments
    /// * `xml_document` - The document to serialize.
    /// * `file_path` - The path where the XML file will be written.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Success or an error.
    pub fn xml_doc_tree_to_file(
        xml_document: &XmlDocument,
        file_path: &str,
    ) -> AnyResult<(), AnyError> {
        Self::xml_doc_tree_to_file_with(xml_document, file_path, &SerializeOptions::default())
    }

    /// Serializes an XML document tree to a file using the provided options.
    ///
    /// # Arguments
    /// * `xml_document` - The document to serialize.
    /// * `file_path` - The path where the XML file will be written.
    /// * `options` - Serialization options controlling namespace optimization.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Success or an error.
    pub fn xml_doc_tree_to_file_with(
        xml_document: &XmlDocument,
        file_path: &str,
        options: &SerializeOptions,
    ) -> AnyResult<(), AnyError> {
        info!(
            "draviavemal-xml_rs::Serializing XML document to file: {}",
            file_path
        );
        // Convert the document to a byte vector
        let xml_bytes = Self::xml_tree_to_vec_with(xml_document, options)?;

        // Write the bytes to the file
        fs::write(file_path, &xml_bytes)
            .map_err(|e| {
                error!(
                    "draviavemal-xml_rs::Failed to write XML file '{}': {}",
                    file_path, e
                );
                e
            })
            .context("draviavemal-xml_rs::Failed to write XML file")?;

        debug!(
            "draviavemal-xml_rs::Wrote {} bytes to file '{}'",
            xml_bytes.len(),
            file_path
        );
        Ok(())
    }

    /// Serializes an XML document tree to a byte vector.
    ///
    /// # Arguments
    /// * `xml_document` - The document to serialize.
    ///
    /// # Returns
    /// * `AnyResult<Vec<u8>, AnyError>` - The serialized XML as bytes, or an error.
    pub fn xml_tree_to_vec(xml_document: &XmlDocument) -> AnyResult<Vec<u8>, AnyError> {
        Self::xml_tree_to_vec_with(xml_document, &SerializeOptions::default())
    }

    /// Serializes an XML document tree to a byte vector using the provided options.
    ///
    /// # Arguments
    /// * `xml_document` - The document to serialize.
    /// * `options` - Serialization options controlling namespace optimization.
    ///
    /// # Returns
    /// * `AnyResult<Vec<u8>, AnyError>` - The serialized XML as bytes, or an error.
    pub fn xml_tree_to_vec_with(
        xml_document: &XmlDocument,
        options: &SerializeOptions,
    ) -> AnyResult<Vec<u8>, AnyError> {
        debug!("draviavemal-xml_rs::Building XML output string from document tree");
        let mut xml_content = String::default();

        // Add XML declaration with conditional behavior based on build mode
        #[cfg(debug_assertions)]
        {
            xml_content.push_str(
                format!(
                    "<?xml version=\"{}\" encoding=\"{}\"",
                    xml_document.get_version(),
                    xml_document.get_encoding()
                )
                .as_str(),
            );
            if let Some(standalone) = xml_document.get_standalone() {
                xml_content.push_str(format!(" standalone=\"{}\"", standalone).as_str());
            }
            xml_content.push_str("?>");
        }

        #[cfg(not(debug_assertions))]
        {
            // In release mode, add XML declaration and metadata comment
            // The metadata includes package info and timestamp
            use chrono::Utc;
            xml_content.push_str(r#"<?xml version="1.0" encoding="utf-8"?>"#);
            xml_content.push_str(
                format!(r#"<!--<dvmo:office><dvmo:appName>{}</dvmo:appName><dvmo:repo>{}</dvmo:repo><dvmo:version>{}</dvmo:version><dvmo:modified>{}</dvmo:modified></dvmo:office>-->"#,
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_REPOSITORY"),
                    env!("CARGO_PKG_VERSION"),
                    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                )
                .as_str(),);
        }

        let namespace_plan = if options.optimize_namespaces {
            Some(
                NamespaceOptimizer::build_plan(xml_document)
                    .context("draviavemal-xml_rs::Failed to build namespace optimization plan")?,
            )
        } else {
            None
        };

        for comment in xml_document.get_prolog_comments() {
            xml_content.push_str("<!--");
            xml_content.push_str(comment);
            xml_content.push_str("-->");
        }

        // Build the XML tree, measuring performance in debug mode
        xml_content.push_str(
            log_elapsed!(
                || {
                    Self::build_xml_tree(xml_document, namespace_plan.as_ref())
                        .context("draviavemal-xml_rs::Create XML Contact String Failed")
                },
                format!("Deserialize File :")
            )?
            .as_str(),
        );

        // Convert the string to UTF-8 bytes
        let xml_bytes = xml_content.as_bytes().to_vec();
        info!(
            "draviavemal-xml_rs::XML document serialized to {} bytes",
            xml_bytes.len()
        );
        Ok(xml_bytes)
    }
}

impl XmlSerializer {
    // --------------------------
    // private methods
    // --------------------------

    /// Appends a single `xmlns` declaration to the tag being built.
    fn push_namespace_declaration(target: &mut String, alias: &str, uri: &str) {
        target.push_str(" xmlns");
        if !alias.is_empty() {
            target.push(':');
            target.push_str(alias);
        }
        target.push_str("=\"");
        target.push_str(uri);
        target.push('"');
    }

    /// Builds the opening tag for an element with its namespace declarations and attributes.
    ///
    /// # Arguments
    /// * `element` - The element to build a tag for.
    /// * `element_id` - The node ID of the element.
    /// * `namespace_plan` - When present, prefixed declarations are emitted from this
    ///   optimization plan instead of the element's own stored declarations.
    ///
    /// # Returns
    /// * `Result<String, AnyError>` - The formatted tag string with declarations and attributes.
    fn build_element(
        element: &XmlElement,
        element_id: NodeId,
        namespace_plan: Option<&NamespacePlan>,
    ) -> Result<String, AnyError> {
        let mut element_part = String::default();
        element_part.push_str(&element.get_tag_ns());

        match namespace_plan {
            Some(plan) => {
                for (alias, uri) in element.get_local_namespace_declarations() {
                    if alias.is_empty() {
                        Self::push_namespace_declaration(&mut element_part, &alias, &uri);
                    }
                }
                if let Some(declarations) = plan.get(&element_id) {
                    for (alias, uri) in declarations {
                        Self::push_namespace_declaration(&mut element_part, alias, uri);
                    }
                }
            }
            None => {
                if element.has_namespace() {
                    for (alias, uri) in element.get_local_namespace_declarations() {
                        Self::push_namespace_declaration(&mut element_part, &alias, &uri);
                    }
                }
            }
        }

        if let Some(attributes) = element.get_attributes() {
            for attribute in attributes {
                element_part.push(' ');
                if let Some(alias) = attribute.get_ns_alias() {
                    if !alias.is_empty() {
                        element_part.push_str(alias);
                        element_part.push(':');
                    }
                }
                element_part.push_str(attribute.get_name());
                element_part.push_str("=\"");
                element_part.push_str(attribute.get_value());
                element_part.push('"');
            }
        }

        Ok(element_part)
    }

    /// Recursively builds the XML content for an element and its children.
    ///
    /// # Arguments
    /// * `xml_document` - The XML document containing all elements.
    /// * `element_id` - The ID of the element to process.
    /// * `namespace_plan` - Optional namespace optimization plan.
    ///
    /// # Returns
    /// * `Result<String, AnyError>` - The serialized element content or an error.
    fn build_element_content(
        xml_document: &XmlDocument,
        element_id: NodeId,
        namespace_plan: Option<&NamespacePlan>,
    ) -> Result<String, AnyError> {
        let mut content_part = String::default();

        // Borrow the element in place; serialization never mutates the tree.
        let element = xml_document
            .get_element(element_id)
            .context("draviavemal-xml_rs::Failed to get element")?;

        // Check if the element has contents
        if let Some(contents) = element.get_child_contents() {
            let open_tag = Self::build_element(element, element_id, namespace_plan)?;
            content_part.push('<');
            content_part.push_str(&open_tag);
            content_part.push('>');

            for content in contents {
                match content {
                    // Recursively process child elements
                    XmlElementContentType::Element((id, _, _)) => {
                        let element_content =
                            Self::build_element_content(xml_document, *id, namespace_plan)
                                .context("draviavemal-xml_rs::Failed to build element content")?;
                        content_part.push_str(&element_content);
                    }
                    // Escape and add text content
                    XmlElementContentType::Text(text) => {
                        content_part.push_str(&escape(text.as_str()));
                    }
                    // Format comments
                    XmlElementContentType::Comment(comment) => {
                        content_part.push_str("<!--");
                        content_part.push_str(comment);
                        content_part.push_str("-->");
                    }
                }
            }

            // Only add closing tag if there's content
            if !contents.is_empty() {
                content_part.push_str("</");
                content_part.push_str(&element.get_tag_ns());
                content_part.push('>');
            }
        } else {
            let open_tag = Self::build_element(element, element_id, namespace_plan)?;
            content_part.push('<');
            content_part.push_str(&open_tag);
            content_part.push_str("/>");
        }

        Ok(content_part)
    }

    /// Builds the complete XML tree starting from the root element.
    ///
    /// # Arguments
    /// * `xml_document` - The XML document to serialize.
    /// * `namespace_plan` - Optional namespace optimization plan.
    ///
    /// # Returns
    /// * `AnyResult<String, AnyError>` - The complete XML string or an error.
    fn build_xml_tree(
        xml_document: &XmlDocument,
        namespace_plan: Option<&NamespacePlan>,
    ) -> AnyResult<String, AnyError> {
        let mut xml_part = String::default();

        // Get the root element ID
        let current_id = xml_document.get_root_id();
        trace!(
            "draviavemal-xml_rs::Serializing XML tree starting from root node {}",
            current_id
        );

        // Build the XML tree starting from the root
        let root_content = Self::build_element_content(xml_document, current_id, namespace_plan)
            .context("draviavemal-xml_rs::Failed to build root content tree")?;

        xml_part.push_str(&root_content);

        Ok(xml_part)
    }
}
