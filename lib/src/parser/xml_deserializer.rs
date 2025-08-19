/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{log_elapsed, NodeId, XmlAttribute, XmlDocument, XmlElementContentType};
use anyhow::{Context, Error as AnyError, Result as AnyResult};
use quick_xml::{
    events::{BytesStart, Event},
    NsReader,
};
use std::{fs, io::Cursor};

/// Deserializer for converting XML data into an XmlDocument object.
///
/// This struct provides functionality to parse XML from files or byte vectors
/// and build a structured XML document representation.
pub struct XmlDeserializer {}

impl XmlDeserializer {
    // --------------------------
    // pub methods
    // --------------------------

    /// Parses an XML file into an XmlDocument tree structure.
    ///
    /// # Arguments
    /// * `file_path` - Path to the XML file to parse.
    ///
    /// # Returns
    /// * `AnyResult<XmlDocument, AnyError>` - The parsed document or an error.
    pub fn file_to_xml_doc_tree(file_path: &str) -> AnyResult<XmlDocument, AnyError> {
        // Read the XML file into a byte vector
        let xml_str = fs::read(file_path).context("Failed to read XML file")?;
        // Delegate to the vector-based parser
        Self::vec_to_xml_doc_tree(xml_str)
    }

    /// Parses an XML byte vector into an XmlDocument tree structure.
    ///
    /// # Arguments
    /// * `xml_str` - XML content as a byte vector.
    ///
    /// # Returns
    /// * `AnyResult<XmlDocument, AnyError>` - The parsed document or an error.
    pub fn vec_to_xml_doc_tree(xml_str: Vec<u8>) -> AnyResult<XmlDocument, AnyError> {
        // Create a reader for the XML content
        let mut reader: NsReader<Cursor<Vec<u8>>> = NsReader::from_reader(Cursor::new(xml_str));
        let mut xml_document = XmlDocument::new();

        // Configure the reader to trim whitespace in text nodes
        reader.config_mut().trim_text(true);

        // Parse the XML content, measuring the elapsed time in debug mode
        log_elapsed!(
            || {
                Self::xml_element_parser(&mut reader, &mut xml_document)
                    .context("Xml Element Parser Failed")
            },
            "Serializing"
        )?;

        Ok(xml_document)
    }
}

impl XmlDeserializer {
    // --------------------------
    // private methods
    // --------------------------

    /// Core XML parsing function that processes the XML events and builds the document structure.
    ///
    /// # Arguments
    /// * `reader` - The XML reader providing events.
    /// * `xml_document` - The document being constructed.
    ///
    /// # Returns
    /// * `AnyResult<(), AnyError>` - Success or an error.
    fn xml_element_parser(
        reader: &mut NsReader<Cursor<Vec<u8>>>,
        xml_document: &mut XmlDocument,
    ) -> AnyResult<(), AnyError> {
        let mut temp_buffer = Vec::new();
        let mut root_loaded = false;
        let mut active_xml_element_id: NodeId = 0;

        // Process XML events until EOF or error
        loop {
            match reader.read_event_into(&mut temp_buffer) {
                Err(e) => return Err(e.into()),

                // Process XML declaration
                Ok(Event::Decl(declaration)) => {
                    // Extract and set version information
                    let version = declaration
                        .version()
                        .map(|char| String::from_utf8_lossy(&char).to_string())
                        .unwrap_or_default();
                    xml_document.set_version_mut(version);

                    // Extract and set encoding information, defaulting to utf-8
                    let encoding = match declaration.encoding() {
                        Some(Ok(enc)) => String::from_utf8_lossy(&enc).to_string(),
                        _ => "utf-8".to_string(),
                    };
                    xml_document.set_encoding_mut(encoding);
                }

                // Process empty elements (self-closing tags)
                Ok(Event::Empty(element)) => {
                    // Extract tag name and attributes
                    let tag = String::from_utf8_lossy(element.name().into_inner()).to_string();
                    let attributes = Self::get_attributes_string(element)?;

                    if root_loaded {
                        // Add as child element to current active element
                        xml_document
                            .append_child_element_mut(active_xml_element_id, &tag, Some(attributes))
                            .context("Insert XML Child Failed.")?;
                    } else {
                        // Set as root element
                        xml_document
                            .create_root_element_mut(&tag, Some(attributes))
                            .context("Create XML Root Element Failed")?;
                        root_loaded = true
                    }
                }

                // Process start of element
                Ok(Event::Start(element)) => {
                    // Extract tag name and attributes
                    let tag = String::from_utf8_lossy(element.name().into_inner()).to_string();
                    let attributes = Self::get_attributes_string(element)?;

                    if root_loaded {
                        // Add as child element to current active element and make it the new active element
                        active_xml_element_id = xml_document
                            .append_child_element_mut(active_xml_element_id, &tag, Some(attributes))
                            .context("Insert XML Child Failed.")?;
                    } else {
                        // Set as root element and make it the active element
                        active_xml_element_id = xml_document
                            .create_root_element_mut(&tag, Some(attributes))
                            .context("Create XML Root Element Failed")?;
                        root_loaded = true
                    }
                }

                // Process text content
                Ok(Event::Text(byte_text)) => {
                    // Unescape and add text to current active element
                    let text = byte_text
                        .unescape()
                        .context("XML Text parsing error")?
                        .to_string();
                    xml_document
                        .get_element_mut(active_xml_element_id)
                        .context("Getting Target Element for text Failed")?
                        .add_child_content_mut(XmlElementContentType::Text(text))
                        .context("Failed to add text content")?;
                }

                // Process comments
                Ok(Event::Comment(byte_comment)) => {
                    // Unescape and add comment to current active element
                    // Comments are preserved in the DOM and will be included during serialization
                    let comment = byte_comment
                        .unescape()
                        .context("XML Comment parsing error")?
                        .to_string();
                    xml_document
                        .get_element_mut(active_xml_element_id)
                        .context("Getting Target Element for comments Failed")?
                        .add_child_content_mut(XmlElementContentType::Comment(comment))
                        .context("Failed to add comments")?;
                }

                // Process end of element
                Ok(Event::End(element)) => {
                    // Extract tag name
                    let tag = String::from_utf8_lossy(element.name().into_inner()).to_string();
                    let element = xml_document
                        .get_element_mut(active_xml_element_id)
                        .context("Invalid XML Tree Parsing Failed.")?;

                    // Verify matching start and end tags
                    if element.get_tag_ns() == tag {
                        // Move back up to parent element
                        if let Some(parent_id) = element.get_parent_id() {
                            active_xml_element_id = parent_id;
                        }
                    } else {
                        // Error if tags don't match
                        return Err(AnyError::msg(format!(
                            "Invalid XML Tree Parsing Failed. Check {} != {}",
                            element.get_tag_ns(),
                            tag
                        )));
                    }
                }

                // End of file reached
                Ok(Event::Eof) => {
                    break;
                }

                // Ignore other events
                _ => {}
            }

            // Clear buffer for next event
            temp_buffer.clear();
        }

        Ok(())
    }

    /// Extracts attributes from an XML element start tag.
    ///
    /// # Arguments
    /// * `element` - The XML element containing attributes.
    ///
    /// # Returns
    /// * `AnyResult<Vec<XmlAttribute>, AnyError>` - Vector of attributes or an error.
    fn get_attributes_string(element: BytesStart) -> AnyResult<Vec<XmlAttribute>, AnyError> {
        // Transform each attribute into an XmlAttribute
        element
            .html_attributes()
            .map(|attribute_result| {
                let attribute = attribute_result.context("Failed to parse attribute")?;
                // Extract name and value
                let name = String::from_utf8_lossy(attribute.key.into_inner()).to_string();
                let value = String::from_utf8_lossy(&attribute.value).to_string();
                // Create the XmlAttribute
                Ok(XmlAttribute::new(name, value))
            })
            .collect::<AnyResult<Vec<XmlAttribute>, AnyError>>()
    }
}
