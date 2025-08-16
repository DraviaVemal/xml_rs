use crate::{NodeId, XmlDocument, XmlElement, XmlElementContentType, log_elapsed};
use anyhow::{Context, Error as AnyError, Result as AnyResult};
use quick_xml::escape::escape;

/// Provides XML serialization utilities for `XmlDocument`.
pub struct XmlSerializer {}

impl XmlSerializer {
    pub fn xml_tree_to_vec(xml_document: &mut XmlDocument) -> AnyResult<Vec<u8>, AnyError> {
        let mut xml_content = String::new();
        #[cfg(debug_assertions)]
        {
            // Add XML declaration in debug mode
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
            // Add XML declaration and metadata comment in release mode.
            // The metadata comment includes package name, repo, version, and current timestamp.
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
        // Use log_elapsed! macro to time and log the serialization process.
        xml_content.push_str(
            log_elapsed!(
                || {
                    Self::build_xml_tree(xml_document).context("Create XML Contact String Failed")
                },
                format!("Deserialize File :")
            )?
            .as_str(),
        );
        // Return the XML as a UTF-8 byte vector.
        Ok(xml_content.as_bytes().to_vec())
    }

    fn build_element(element: &XmlElement) -> String {
        let mut element_part = String::new();
        element_part.push_str(&element.get_tag_ns());
        if let Some(attributes) = element.get_attributes() {
            for attribute in attributes {
                element_part.push_str(&format!(
                    " {}=\"{}\"",
                    attribute.get_ns_name(),
                    attribute.get_value()
                ));
            }
        }
        element_part
    }

    fn build_element_content(
        xml_document: &mut XmlDocument,
        element_id: &NodeId,
    ) -> Result<String, AnyError> {
        let mut content_part = String::new();
        let element = xml_document
            .get_element_mut(element_id)
            .context("Failed to get element")?
            .clone();
        if let Some(contents) = element.get_contents() {
            content_part.push_str(&format!("<{}>", Self::build_element(&element)));
            for content in contents {
                match content {
                    XmlElementContentType::Element((id, _)) => {
                        let element_content = Self::build_element_content(xml_document, &id)
                            .context("Failed to build element content")?;
                        content_part.push_str(&element_content);
                    }
                    XmlElementContentType::Text(text) => {
                        content_part.push_str(&escape(text.to_string()));
                    }
                    XmlElementContentType::Comment(comment) => {
                        content_part.push_str(&format!("<!--{}-->", comment));
                    }
                }
            }
            if contents.len() > 0 {
                content_part.push_str(&format!("</{}>", element.get_tag_ns()));
            }
        } else {
            content_part.push_str(&format!("<{}/>", Self::build_element(&element)));
        }
        Ok(content_part)
    }

    fn build_xml_tree(xml_document: &mut XmlDocument) -> AnyResult<String, AnyError> {
        let mut xml_part = String::new();
        let current_id = *xml_document.get_root_id();
        let test = Self::build_element_content(xml_document, &current_id)
            .context("Failed to build root content tree")?;
        xml_part.push_str(&test);
        Ok(xml_part)
    }
}
