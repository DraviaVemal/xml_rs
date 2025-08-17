/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{log_elapsed, NodeId, XmlDocument, XmlElement, XmlElementContentType};
use anyhow::{Context, Error as AnyError, Result as AnyResult};
use quick_xml::escape::escape;
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
        // Convert the document to a byte vector
        let xml_bytes = Self::xml_tree_to_vec(xml_document)?;
        
        // Write the bytes to the file
        fs::write(file_path, xml_bytes).context("Failed to write XML file")?;
        
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
        let mut xml_content = String::new();
        
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
                    Self::build_xml_tree(xml_document).context("Create XML Contact String Failed")
                },
                format!("Deserialize File :")
            )?
            .as_str(),
        );
        
        // Convert the string to UTF-8 bytes
        Ok(xml_content.as_bytes().to_vec())
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
    /// * `String` - The formatted tag string with attributes.
    fn build_element(element: &XmlElement) -> String {
        let mut element_part = String::new();
        
        // Add the tag name with namespace if present
        element_part.push_str(&element.get_tag_ns());
        
        // Add attributes if present
        if let Some(attributes) = element.get_attributes() {
            for attribute in attributes {
                // Format each attribute as 'name="value"'
                element_part.push_str(&format!(
                    " {}=\"{}\"",
                    attribute.get_ns_name(),
                    attribute.get_value()
                ));
            }
        }
        
        element_part
    }

    /// Recursively builds the XML content for an element and its children.
    ///
    /// # Arguments
    /// * `xml_document` - The XML document containing all elements.
    /// * `element_id` - The ID of the element to process.
    ///
    /// # Returns
    /// * `Result<String, AnyError>` - The serialized element content or an error.
    fn build_element_content(
        xml_document: &mut XmlDocument,
        element_id: NodeId,
    ) -> Result<String, AnyError> {
        let mut content_part = String::new();
        
        // Get a copy of the element to work with
        let element = xml_document
            .get_element_mut(element_id)
            .context("Failed to get element")?
            .clone_limited();
        
        // Check if the element has contents
        if let Some(contents) = element.get_contents() {
            // Start tag with attributes
            content_part.push_str(&format!("<{}>", Self::build_element(&element)));
            
            // Process each content item
            for content in contents {
                match content {
                    // Recursively process child elements
                    XmlElementContentType::Element((id, _)) => {
                        let element_content = Self::build_element_content(xml_document, *id)
                            .context("Failed to build element content")?;
                        content_part.push_str(&element_content);
                    },
                    // Escape and add text content
                    XmlElementContentType::Text(text) => {
                        content_part.push_str(&escape(text.to_string()));
                    },
                    // Format comments
                    XmlElementContentType::Comment(comment) => {
                        content_part.push_str(&format!("<!--{}-->", comment));
                    },
                }
            }
            
            // Only add closing tag if there's content
            if !contents.is_empty() {
                content_part.push_str(&format!("</{}>", element.get_tag_ns()));
            }
        } else {
            // Self-closing tag for elements without content
            content_part.push_str(&format!("<{}/>", Self::build_element(&element)));
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
        let mut xml_part = String::new();
        
        // Get the root element ID
        let current_id = xml_document.get_root_id();
        
        // Build the XML tree starting from the root
        let root_content = Self::build_element_content(xml_document, current_id)
            .context("Failed to build root content tree")?;
        
        xml_part.push_str(&root_content);
        
        Ok(xml_part)
    }
}
