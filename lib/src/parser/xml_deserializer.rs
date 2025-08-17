use crate::{log_elapsed, NodeId, XmlAttribute, XmlDocument, XmlElementContentType};
use anyhow::{Context, Error as AnyError, Result as AnyResult};
use quick_xml::{events::BytesStart, events::Event, NsReader};
use std::{fs, io::Cursor};

pub struct XmlDeserializer {}

impl XmlDeserializer {
    
    pub fn file_to_xml_doc_tree(file_path: &str) -> AnyResult<XmlDocument, AnyError> {
        let xml_str = fs::read(file_path).context("Failed to read XML file")?;
        Self::vec_to_xml_doc_tree(xml_str)
    }

    pub fn vec_to_xml_doc_tree(xml_str: Vec<u8>) -> AnyResult<XmlDocument, AnyError> {
        let mut reader: NsReader<Cursor<Vec<u8>>> = NsReader::from_reader(Cursor::new(xml_str));
        let mut xml_document = XmlDocument::new();
        reader.config_mut().trim_text(true);
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
    fn xml_element_parser(
        reader: &mut NsReader<Cursor<Vec<u8>>>,
        xml_document: &mut XmlDocument,
    ) -> AnyResult<(), AnyError> {
        let mut temp_buffer = Vec::new();
        let mut root_loaded = false;
        let mut active_xml_element_id: NodeId = 0;
        loop {
            match reader.read_event_into(&mut temp_buffer) {
                Err(e) => return Err(e.into()),
                Ok(Event::Decl(declaration)) => {
                    let version = declaration
                        .version()
                        .map(|char| String::from_utf8_lossy(&char).to_string())
                        .unwrap_or_default();
                    xml_document.set_version_mut(version);
                    let encoding = match declaration.encoding() {
                        Some(Ok(enc)) => String::from_utf8_lossy(&enc).to_string(),
                        _ => "utf-8".to_string(),
                    };
                    xml_document.set_encoding_mut(encoding);
                }
                Ok(Event::Empty(element)) => {
                    let tag = String::from_utf8_lossy(element.name().into_inner()).to_string();
                    let attributes = Self::get_attributes_string(element)?;
                    if root_loaded {
                        xml_document
                            .append_child_element_mut(active_xml_element_id, &tag, Some(attributes))
                            .context("Insert XML Child Failed.")?;
                    } else {
                        xml_document
                            .create_root_element_mut(tag, Some(attributes))
                            .context("Create XML Root Element Failed")?;
                        root_loaded = true
                    }
                }
                Ok(Event::Start(element)) => {
                    let tag = String::from_utf8_lossy(element.name().into_inner()).to_string();
                    let attributes = Self::get_attributes_string(element)?;
                    if root_loaded {
                        active_xml_element_id = xml_document
                            .append_child_element_mut(active_xml_element_id, &tag, Some(attributes))
                            .context("Insert XML Child Failed.")?;
                    } else {
                        active_xml_element_id = xml_document
                            .create_root_element_mut(tag, Some(attributes))
                            .context("Create XML Root Element Failed")?;
                        root_loaded = true
                    }
                }
                Ok(Event::Text(byte_text)) => {
                    let text = byte_text
                        .unescape()
                        .context("XML Text parsing error")?
                        .to_string();
                    xml_document
                        .get_element_mut(active_xml_element_id)
                        .context("Getting Target Element Failed")?
                        .add_content_mut(XmlElementContentType::Text(text));
                }
                Ok(Event::Comment(byte_comment)) => {
                    let comment = byte_comment
                        .unescape()
                        .context("XML Text parsing error")?
                        .to_string();
                    xml_document
                        .get_element_mut(active_xml_element_id)
                        .context("Getting Target Element Failed")?
                        .add_content_mut(XmlElementContentType::Comment(comment));
                }
                Ok(Event::End(element)) => {
                    let tag = String::from_utf8_lossy(element.name().into_inner()).to_string();
                    let element = xml_document
                        .get_element_mut(active_xml_element_id)
                        .context("Invalid XML Tree Parsing Failed.")?;
                    if element.get_tag_ns() == tag {
                        if let Some(parent_id) = element.get_parent_id() {
                            active_xml_element_id = parent_id;
                        }
                    } else {
                        return Err(AnyError::msg(format!(
                            "Invalid XML Tree Parsing Failed. Check {} != {}",
                            element.get_tag_ns(),
                            tag
                        )));
                    }
                }
                Ok(Event::Eof) => {
                    break;
                }
                _ => {}
            }
            temp_buffer.clear();
        }
        Ok(())
    }

    fn get_attributes_string(element: BytesStart) -> AnyResult<Vec<XmlAttribute>, AnyError> {
        element
            .html_attributes()
            .map(|attribute_result| {
                let attribute = attribute_result.context("Failed to parse attribute")?;
                let name = String::from_utf8_lossy(attribute.key.into_inner()).to_string();
                let value = String::from_utf8_lossy(&attribute.value).to_string();
                Ok(XmlAttribute::new(name, value))
            })
            .collect::<AnyResult<Vec<XmlAttribute>, AnyError>>()
    }
}
