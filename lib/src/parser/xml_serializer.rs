/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{log_elapsed, NodeId, XmlDocument, XmlElement, XmlElementContentType};
use anyhow::{Context, Error as AnyError, Result as AnyResult};
use log::{debug, error, info, trace};
use quick_xml::escape::escape;
use std::collections::HashMap;
use std::fs;

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
        xml_document: &mut XmlDocument,
        file_path: &str,
    ) -> AnyResult<(), AnyError> {
        info!(
            "draviavemal-xml_rs::Serializing XML document to file: {}",
            file_path
        );
        // Convert the document to a byte vector
        let xml_bytes = Self::xml_tree_to_vec(xml_document)?;

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
    pub fn xml_tree_to_vec(xml_document: &mut XmlDocument) -> AnyResult<Vec<u8>, AnyError> {
        debug!("draviavemal-xml_rs::Building XML output string from document tree");
        let mut xml_content = String::default();

        // Add XML declaration with conditional behavior based on build mode
        #[cfg(debug_assertions)]
        {
            // Add XML declaration in debug mode with document's version and encoding
            xml_content.push_str(
                format!(
                    "<?xml version=\"{}\" encoding=\"{}\"?>",
                    xml_document.get_version(),
                    xml_document.get_encoding()
                )
                .as_str(),
            );
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

        // Build the XML tree, measuring performance in debug mode
        xml_content.push_str(
            log_elapsed!(
                || {
                    Self::build_xml_tree(xml_document)
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

    /// Builds the opening tag for an element with its attributes.
    ///
    /// # Arguments
    /// * `element` - The element to build a tag for.
    ///
    /// # Returns
    /// * `Result<String, AnyError>` - The formatted tag string with attributes.
    fn build_element(
        element: &XmlElement,
        inherited_ns: &HashMap<String, String>,
    ) -> Result<(String, Vec<(String, String)>), AnyError> {
        let mut element_part = String::default();
        let mut emitted_ns = Vec::new();

        element_part.push_str(&element.get_tag_ns());

        if element.has_namespace() {
            let namespace_context = element.get_namespace_context();
            let namespace = namespace_context
                .try_borrow()
                .context("draviavemal-xml_rs::Failed to borrow namespace context")?;
            for (prefix, (uri, usage_count)) in namespace.get_namespace_alias_url().iter() {
                if *usage_count == 0 {
                    continue;
                }
                if inherited_ns
                    .get(prefix)
                    .map(|bound_uri| bound_uri == uri)
                    .unwrap_or(false)
                {
                    continue;
                }
                element_part.push_str(&format!(
                    " xmlns{}=\"{}\"",
                    if prefix.is_empty() {
                        "".to_string()
                    } else {
                        format!(":{}", prefix).to_string()
                    },
                    uri
                ));
                emitted_ns.push((prefix.clone(), uri.clone()));
            }
        }
        if let Some(attributes) = element.get_attributes() {
            for attribute in attributes {
                element_part.push_str(&format!(
                    " {}=\"{}\"",
                    attribute.get_ns_name(),
                    attribute.get_value()
                ));
            }
        }

        Ok((element_part, emitted_ns))
    }

    /// Recursively builds the XML content for an element and its children.
    ///
    /// # Arguments
    /// * `xml_document` - The XML document containing all elements.
    /// * `element_id` - The ID of the element to process.
    /// * `inherited_ns` - Prefix-to-URI bindings already emitted by ancestor elements.
    ///
    /// # Returns
    /// * `Result<String, AnyError>` - The serialized element content or an error.
    fn build_element_content(
        xml_document: &mut XmlDocument,
        element_id: NodeId,
        inherited_ns: &HashMap<String, String>,
    ) -> Result<String, AnyError> {
        let mut content_part = String::default();

        // Get a copy of the element to work with
        let element = xml_document
            .get_element_mut(element_id)
            .context("draviavemal-xml_rs::Failed to get element")?
            .clone_limited();

        // Check if the element has contents
        if let Some(contents) = element.get_child_contents() {
            let (open_tag, emitted_ns) = Self::build_element(&element, inherited_ns)?;
            content_part.push_str(&format!("<{}>", open_tag));

            let mut extended_scope;
            let child_ns: &HashMap<String, String> = if emitted_ns.is_empty() {
                inherited_ns
            } else {
                extended_scope = inherited_ns.clone();
                extended_scope.extend(emitted_ns);
                &extended_scope
            };

            for content in contents {
                match content {
                    // Recursively process child elements
                    XmlElementContentType::Element((id, _, _)) => {
                        let element_content =
                            Self::build_element_content(xml_document, *id, child_ns)
                                .context("draviavemal-xml_rs::Failed to build element content")?;
                        content_part.push_str(&element_content);
                    }
                    // Escape and add text content
                    XmlElementContentType::Text(text) => {
                        content_part.push_str(&escape(text.to_string()));
                    }
                    // Format comments
                    XmlElementContentType::Comment(comment) => {
                        content_part.push_str(&format!("<!--{}-->", comment));
                    }
                }
            }

            // Only add closing tag if there's content
            if !contents.is_empty() {
                content_part.push_str(&format!("</{}>", element.get_tag_ns()));
            }
        } else {
            let (open_tag, _) = Self::build_element(&element, inherited_ns)?;
            content_part.push_str(&format!("<{}/>", open_tag));
        }

        Ok(content_part)
    }

    /// Builds the complete XML tree starting from the root element.
    ///
    /// # Arguments
    /// * `xml_document` - The XML document to serialize.
    ///
    /// # Returns
    /// * `AnyResult<String, AnyError>` - The complete XML string or an error.
    fn build_xml_tree(xml_document: &mut XmlDocument) -> AnyResult<String, AnyError> {
        let mut xml_part = String::default();

        // Get the root element ID
        let current_id = xml_document.get_root_id();
        trace!(
            "draviavemal-xml_rs::Serializing XML tree starting from root node {}",
            current_id
        );

        // Build the XML tree starting from the root
        let root_content = Self::build_element_content(xml_document, current_id, &HashMap::new())
            .context("draviavemal-xml_rs::Failed to build root content tree")?;

        xml_part.push_str(&root_content);

        Ok(xml_part)
    }
}
