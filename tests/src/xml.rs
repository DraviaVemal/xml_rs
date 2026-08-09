#[cfg(test)]
mod xml_test {
    use draviavemal_xml_rs::{
        NamespaceDeclaration, SerializeOptions, XmlAttribute, XmlDeserializer, XmlDocument,
        XmlElementContentType, XmlSerializer,
    };

    /// Test data for common XML test cases
    fn get_test_xml() -> &'static str {
        r#"
        <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <!-- This is Test Content 1 -->
        <!-- This is Test Content 2 -->
        <workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="x15 xr xr6 xr10 xr2" xmlns:x15="http://schemas.microsoft.com/office/spreadsheetml/2010/11/main" xmlns:xr="http://schemas.microsoft.com/office/spreadsheetml/2014/revision" xmlns:xr6="http://schemas.microsoft.com/office/spreadsheetml/2016/revision6" xmlns:xr10="http://schemas.microsoft.com/office/spreadsheetml/2016/revision10" xmlns:xr2="http://schemas.microsoft.com/office/spreadsheetml/2015/revision2">
        <fileVersion appName="xl" lastEdited="7" lowestEdited="7" rupBuild="28827"/>
            <!-- This is Test Content 3 -->
            <!-- This is Test Content 4 -->
        <workbookPr defaultThemeVersion="166925"/>
        <mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">
            <mc:Choice Requires="x15">
            <x15ac:absPath url="https://medtronicapac-my.sharepoint.com/personal/md10_medtronic_com/Documents/Desktop/" xmlns:x15ac="http://schemas.microsoft.com/office/spreadsheetml/2010/11/ac"/>
            </mc:Choice>
        </mc:AlternateContent>
        <xr:revisionPtr revIDLastSave="21" documentId="13_ncr:1_{A17606B1-6AE4-4585-A1ED-53244B14AA1A}" xr6:coauthVersionLast="47" xr6:coauthVersionMax="47" xr10:uidLastSave="{C4F19BA3-024F-4648-B4F0-8FB9AB5DD2C8}"/>
        <bookViews>
            <workbookView xWindow="28680" yWindow="-120" windowWidth="29040" windowHeight="15720" activeTab="2" xr2:uid="{55A69094-CFF0-4A24-937F-3F8966A07938}"/>
        </bookViews>
        <sheets>
            <sheet name="Style" sheetId="1" r:id="rId1"/>
            <sheet name="formula" sheetId="2" r:id="rId2"/>
            <sheet name="image" sheetId="3" r:id="rId3"/>
        </sheets>
        <calcPr calcId="191029"/>
        <extLst>
            <ext uri="{140A7094-0E35-4892-8432-C4D2E57EDEB5}" xmlns:x15="http://schemas.microsoft.com/office/spreadsheetml/2010/11/main">
            <x15:workbookPr chartTrackingRefBase="1"/>
            </ext>
            <ext uri="{B58B0392-4F1F-4190-BB64-5DF3571DCE5F}" xmlns:xcalcf="http://schemas.microsoft.com/office/spreadsheetml/2018/calcfeatures">
            <xcalcf:calcFeatures>
                <xcalcf:feature name="microsoft.com:RD"/>
                <xcalcf:feature name="microsoft.com:Single"/>
                <xcalcf:feature name="microsoft.com:FV"/>
                <xcalcf:feature name="microsoft.com:CNMTM"/>
                <xcalcf:feature name="microsoft.com:LET_WF"/>
            </xcalcf:calcFeatures>
            </ext>
        </extLst>
        </workbook>
    "#
    }

    fn split_tag_attributes(attribute_region: &str) -> Vec<String> {
        let characters: Vec<char> = attribute_region.chars().collect();
        let mut attributes = Vec::new();
        let mut index = 0;
        while index < characters.len() {
            while index < characters.len() && characters[index].is_whitespace() {
                index += 1;
            }
            if index >= characters.len() || characters[index] == '/' {
                break;
            }
            let name_start = index;
            while index < characters.len() && characters[index] != '=' {
                index += 1;
            }
            if index >= characters.len() {
                break;
            }
            let name: String = characters[name_start..index].iter().collect();
            index += 1;
            let quote = characters[index];
            index += 1;
            let value_start = index;
            while index < characters.len() && characters[index] != quote {
                index += 1;
            }
            let value: String = characters[value_start..index].iter().collect();
            index += 1;
            attributes.push(format!("{}={}{}{}", name.trim(), quote, value, quote));
        }
        attributes.sort();
        attributes
    }

    fn normalize_markup(chunk: &str) -> String {
        if chunk.starts_with("<!") || chunk.starts_with("<?") {
            return chunk.to_string();
        }
        let inner = &chunk[1..chunk.len().saturating_sub(1)];
        if inner.starts_with('/') {
            return chunk.to_string();
        }
        let self_closing = inner.ends_with('/');
        let inner_trimmed = inner.trim_end_matches('/');
        let name_end = inner_trimmed
            .find(char::is_whitespace)
            .unwrap_or(inner_trimmed.len());
        let name = &inner_trimmed[..name_end];
        let attributes = split_tag_attributes(&inner_trimmed[name_end..]);
        let mut rebuilt = String::from("<");
        rebuilt.push_str(name);
        for attribute in attributes {
            rebuilt.push(' ');
            rebuilt.push_str(&attribute);
        }
        if self_closing {
            rebuilt.push('/');
        }
        rebuilt.push('>');
        rebuilt
    }

    fn canonicalize(input: &str) -> String {
        let characters: Vec<char> = input.chars().collect();
        let mut result = String::new();
        let mut index = 0;
        while index < characters.len() {
            if characters[index] == '<' {
                let mut end = index;
                let mut inside_quote = false;
                while end < characters.len() {
                    let current = characters[end];
                    if current == '"' {
                        inside_quote = !inside_quote;
                    }
                    if current == '>' && !inside_quote {
                        break;
                    }
                    end += 1;
                }
                let chunk: String = characters[index..=end.min(characters.len() - 1)]
                    .iter()
                    .collect();
                result.push_str(&normalize_markup(&chunk));
                index = end + 1;
            } else {
                result.push(characters[index]);
                index += 1;
            }
        }
        result.chars().filter(|c| !c.is_whitespace()).collect()
    }

    fn first_divergence(original: &str, modified: &str) -> Option<(usize, String, String)> {
        let original_significant: Vec<char> = original.chars().collect();
        let modified_significant: Vec<char> = modified.chars().collect();
        let shorter_length = original_significant.len().min(modified_significant.len());
        let mut index = 0;
        while index < shorter_length
            && original_significant[index] == modified_significant[index]
        {
            index += 1;
        }
        if index == original_significant.len() && index == modified_significant.len() {
            return None;
        }
        let context_start = index.saturating_sub(40);
        let original_window: String = original_significant
            [context_start..(index + 60).min(original_significant.len())]
            .iter()
            .collect();
        let modified_window: String = modified_significant
            [context_start..(index + 60).min(modified_significant.len())]
            .iter()
            .collect();
        Some((index, original_window, modified_window))
    }

    #[test]
    fn test_xml_roundtrip_string() {
        let source_content = get_test_xml();
        let xml_doc = XmlDeserializer::vec_to_xml_doc_tree(source_content.as_bytes().to_vec())
            .expect("Failed to parse string to document");
        let xml_vec =
            XmlSerializer::xml_tree_to_vec(&xml_doc).expect("Failed to parse document to string");
        let roundtrip_content = String::from_utf8(xml_vec).expect("Failed to conver vec to string");
        let canonical_source = canonicalize(source_content);
        let canonical_roundtrip = canonicalize(roundtrip_content.as_str());
        if let Some((index, original_window, modified_window)) =
            first_divergence(canonical_source.as_str(), canonical_roundtrip.as_str())
        {
            panic!(
                "Round-trip diverged at character {}\n  source : ...{}\n  output : ...{}",
                index, original_window, modified_window
            );
        }
    }

    #[test]
    fn test_xml_deserialization() {
        let xml_input = get_test_xml();

        // Parse the XML into a document object
        let mut document = XmlDeserializer::vec_to_xml_doc_tree(xml_input.as_bytes().to_vec())
            .expect("Failed to deserialize XML");

        // Serialize the document back to XML
        let content =
            XmlSerializer::xml_tree_to_vec(&mut document).expect("Failed to serialize XML");

        // The test currently only checks that the operations don't panic
        // In a real test, we'd verify the content is correct
        assert!(content.len() > 0, "Serialized XML should not be empty");
    }

    #[test]
    fn test_round_trip_serialization() {
        let xml_input = get_test_xml();

        // First parse the XML
        let mut document = XmlDeserializer::vec_to_xml_doc_tree(xml_input.as_bytes().to_vec())
            .expect("Failed to deserialize XML");

        // Then serialize it back
        let xml_output =
            XmlSerializer::xml_tree_to_vec(&mut document).expect("Failed to serialize XML");
        // Then parse again to ensure it's valid XML
        let document2 = XmlDeserializer::vec_to_xml_doc_tree(xml_output)
            .expect("Failed to deserialize round-tripped XML");

        // Verify that we have elements in the document
        let root_id = document2.get_root_id();
        assert!(root_id > 0, "Root ID should be positive");
    }

    #[test]
    fn test_element_access() {
        let xml_input = get_test_xml();

        // Parse the XML
        let document = XmlDeserializer::vec_to_xml_doc_tree(xml_input.as_bytes().to_vec())
            .expect("Failed to deserialize XML");

        // Get the root element
        let root_id = document.get_root_id();
        let root = document
            .get_element(root_id)
            .expect("Failed to get root element");

        assert_eq!(root.get_tag(), "workbook", "Root tag should be 'workbook'");

        let sheets_id = document
            .find_first_child(root_id, "sheets")
            .expect("Failed to find sheets")
            .expect("No sheets element found");

        let sheet_ids = document
            .find_all_child(sheets_id, "sheet")
            .expect("Failed to find sheets")
            .expect("No sheet elements found");

        assert_eq!(sheet_ids.len(), 3, "Should have 3 sheet elements");

        let first_sheet = document
            .get_element(sheet_ids[0])
            .expect("Failed to get first sheet");

        if let Some(attr) = first_sheet.get_attribute("name") {
            assert_eq!(attr.get_value(), "Style", "First sheet should have name='Style'");
        } else {
            panic!("First sheet should have attributes");
        }
    }

    #[test]
    fn test_find_by_attribute() {
        let xml_input = get_test_xml();

        // Parse the XML
        let document = XmlDeserializer::vec_to_xml_doc_tree(xml_input.as_bytes().to_vec())
            .expect("Failed to deserialize XML");

        // Get the root element
        let root_id = document.get_root_id();

        let sheets_id = document
            .find_first_child(root_id, "sheets")
            .expect("Failed to find sheets")
            .expect("No sheets element found");

        let sheet_id = document
            .find_first_by_attribute(sheets_id, "name", "formula")
            .expect("Failed to search for attribute")
            .expect("No sheet with name='formula' found");

        let sheet = document
            .get_element(sheet_id)
            .expect("Failed to get sheet element");

        assert_eq!(
            sheet.get_attribute("sheetId").unwrap().get_value(),
            "2",
            "formula sheet should have sheetId='2'"
        );
        assert_eq!(
            sheet.get_attribute("id").unwrap().get_value(),
            "rId2",
            "formula sheet should have r:id='rId2'"
        );
    }

    #[test]
    fn test_element_manipulation() {
        // Create a new document
        let mut document = XmlDocument::new();

        // Create root element
        let root_id = document
            .create_root_element_mut(
                "root",
                Some(vec![XmlAttribute::new(
                    "version".to_string(),
                    "1.0".to_string(),
                )]),
            )
            .expect("Failed to create root element");

        // Add a child element
        let child_id = document
            .append_child_element_mut(
                root_id,
                "child",
                Some(vec![XmlAttribute::new(
                    "id".to_string(),
                    "child1".to_string(),
                )]),
            )
            .expect("Failed to append child element");

        let element = document
            .get_element_mut(child_id)
            .expect("Failed to get text element");
        element
            .add_text_mut("Hello World")
            .expect("Failed to add text");

        // Since we can't directly add text, we'll verify differently later

        // Verify the structure
        let root = document
            .get_element(root_id)
            .expect("Failed to get root element");
        assert_eq!(root.get_tag(), "root", "Root tag should be 'root'");

        if let Some(attr) = root.get_attribute("version") {
            assert_eq!(attr.get_value(), "1.0", "Root should have version='1.0'");
        } else {
            panic!("Root should have attributes");
        }

        // Verify the child element exists in root's contents
        if let Some(contents) = root.get_child_contents() {
            let has_child = contents.iter().any(|content| {
                if let XmlElementContentType::Element((id, tag, _)) = content {
                    *id == child_id && tag == "child"
                } else {
                    false
                }
            });

            assert!(has_child, "Root should contain the child element");
        } else {
            panic!("Root should have contents");
        }

        // Check that we can find the child by ID
        let child = document
            .get_element(child_id)
            .expect("Failed to get child element");
        assert_eq!(child.get_tag(), "child", "Child tag should be 'child'");

        // Check the child's text content
        if let Some(contents) = child.get_child_contents() {
            let has_text = contents.iter().any(|content| {
                if let XmlElementContentType::Text(text) = content {
                    text == "Hello World"
                } else {
                    false
                }
            });

            assert!(has_text, "Child should contain 'Hello World' text");
        } else {
            panic!("Child should have contents");
        }
    }

    #[test]
    fn test_remove_element() {
        // Create a new document
        let mut document = XmlDocument::new();

        // Create root element
        let root_id = document
            .create_root_element_mut("root", None)
            .expect("Failed to create root element");

        // Add two child elements
        let child1_id = document
            .append_child_element_mut(
                root_id,
                "child",
                Some(vec![XmlAttribute::new("id".to_string(), "1".to_string())]),
            )
            .expect("Failed to append first child element");

        let child2_id = document
            .append_child_element_mut(
                root_id,
                "child",
                Some(vec![XmlAttribute::new("id".to_string(), "2".to_string())]),
            )
            .expect("Failed to append second child element");

        // Find all children
        let children_before = document
            .find_all_child(root_id, "child")
            .expect("Failed to find children")
            .expect("No children found");

        assert_eq!(
            children_before.len(),
            2,
            "Should have 2 children before removal"
        );

        // Remove the first child
        document
            .remove_element_mut(child1_id)
            .expect("Failed to remove element");

        // Check that only one child remains
        let children_after = document
            .find_all_child(root_id, "child")
            .expect("Failed to find children")
            .expect("No children found after removal");

        assert_eq!(children_after.len(), 1, "Should have 1 child after removal");
        assert_eq!(
            children_after[0], child2_id,
            "Remaining child should be child2"
        );
    }

    #[test]
    fn test_malformed_xml() {
        let malformed_xml = r#"
        <root>
            <unclosed>
                <child>content</child>
            <!-- Missing closing tag for unclosed -->
        </root>
        "#;

        let result = XmlDeserializer::vec_to_xml_doc_tree(malformed_xml.as_bytes().to_vec());
        assert!(result.is_err(), "Parsing malformed XML should fail");
    }

    #[test]
    fn test_empty_document() {
        // Create an empty document
        let mut document = XmlDocument::new();

        // It should fail to serialize without a root element
        let result = XmlSerializer::xml_tree_to_vec(&mut document);
        assert!(
            result.is_err(),
            "Serializing document without root should fail"
        );

        // Add a root element
        document
            .create_root_element_mut("root", None)
            .expect("Failed to create root element");

        // Now it should serialize successfully
        let result = XmlSerializer::xml_tree_to_vec(&mut document);
        assert!(
            result.is_ok(),
            "Serializing document with root should succeed"
        );
    }

    #[test]
    fn test_special_characters() {
        // Create a document with special characters
        let mut document = XmlDocument::new();

        // Create root element
        let root_id = document
            .create_root_element_mut("root", None)
            .expect("Failed to create root element");

        // Add a child element with special characters in text
        let special_id = document
            .append_child_element_mut(root_id, "special", None)
            .expect("Failed to append child element");

        // Add text with special characters
        document
            .get_element_mut(special_id)
            .expect("Failed to get special element")
            .add_text_mut("a<>&\"'")
            .expect("Failed to add text to element");
        document
            .append_child_element_mut(root_id, "special", None)
            .expect("Failed to append special element");

        // Serialize the document to verify special character handling

        // Serialize the document
        let xml_bytes = XmlSerializer::xml_tree_to_vec(&mut document)
            .expect("Failed to serialize document with special characters");

        // Parse it back
        let parsed_doc = XmlDeserializer::vec_to_xml_doc_tree(xml_bytes)
            .expect("Failed to parse document with special characters");

        // Verify the special characters were preserved
        let parsed_root_id = parsed_doc.get_root_id();
        let special_id = parsed_doc
            .find_first_child(parsed_root_id, "special")
            .expect("Failed to find special element")
            .expect("No special element found");

        let special = parsed_doc
            .get_element(special_id)
            .expect("Failed to get special element");

        if let Some(contents) = special.get_child_contents() {
            let has_special_text = contents.iter().any(|content| {
                if let XmlElementContentType::Text(text) = content {
                    text.contains("a<>&\"'")
                } else {
                    false
                }
            });

            assert!(has_special_text, "Special characters should be preserved");
        } else {
            panic!("Special element should have contents");
        }
    }

    #[test]
    fn test_attribute_manipulation() {
        // Create a new document with an element that has attributes
        let mut document = XmlDocument::new();

        // Create root element
        let root_id = document
            .create_root_element_mut("root", None)
            .expect("Failed to create root element");

        // Add a child element with multiple attributes
        let element_id = document
            .append_child_element_mut(
                root_id,
                "element",
                Some(vec![
                    XmlAttribute::new("id".to_string(), "123".to_string()),
                    XmlAttribute::new("class".to_string(), "test-class".to_string()),
                ]),
            )
            .expect("Failed to append element");

        // Get a reference to test initial attributes
        let element = document
            .get_element(element_id)
            .expect("Failed to get element");

        // Verify initial attributes
        assert_eq!(element.get_attribute("id").unwrap().get_value(), "123");
        assert_eq!(
            element.get_attribute("class").unwrap().get_value(),
            "test-class"
        );

        // Modify the element's attributes
        let element = document
            .get_element_mut(element_id)
            .expect("Failed to get mutable element");

        // Add a new attribute
        element
            .add_attribute_mut(XmlAttribute::new(
                "data-test".to_string(),
                "value".to_string(),
            ))
            .expect("Failed to add attribute");

        // Remove an attribute
        element.remove_attribute_mut("class");

        // Verify changes
        let element = document
            .get_element(element_id)
            .expect("Failed to get updated element");

        assert_eq!(element.get_attribute("id").unwrap().get_value(), "123");
        assert!(
            element.get_attribute("class").is_none(),
            "class attribute should be removed"
        );
        assert_eq!(
            element.get_attribute("data-test").unwrap().get_value(),
            "value"
        );

        // Test clear_attribute_mut
        let element = document
            .get_element_mut(element_id)
            .expect("Failed to get mutable element");

        element
            .clear_attribute_mut()
            .expect("Failed to clear attributes");

        // Verify all attributes are gone
        let element = document
            .get_element(element_id)
            .expect("Failed to get updated element");

        assert!(
            element.get_attribute("id").is_none(),
            "All attributes should be cleared"
        );
        assert!(
            element.get_attribute("data-test").is_none(),
            "All attributes should be cleared"
        );
    }

    #[test]
    fn test_comment_handling() {
        // Create a document with comments
        let mut document = XmlDocument::new();

        // Create root element
        let root_id = document
            .create_root_element_mut("root", None)
            .expect("Failed to create root element");

        // Add a child element
        let element_id = document
            .append_child_element_mut(root_id, "element", None)
            .expect("Failed to append element");

        // Add comments to the element
        let element = document
            .get_element_mut(element_id)
            .expect("Failed to get mutable element");

        element
            .add_comments_mut("This is a test comment")
            .expect("Failed to add comment");

        element
            .add_text_mut("Some text content")
            .expect("Failed to add text");

        element
            .add_comments_mut("This is another comment")
            .expect("Failed to add second comment");

        // Serialize the document
        let xml_bytes = XmlSerializer::xml_tree_to_vec(&mut document)
            .expect("Failed to serialize document with comments");

        // Parse it back
        let parsed_doc = XmlDeserializer::vec_to_xml_doc_tree(xml_bytes)
            .expect("Failed to parse document with comments");

        // Verify comments were preserved
        let parsed_root_id = parsed_doc.get_root_id();
        let element_id = parsed_doc
            .find_first_child(parsed_root_id, "element")
            .expect("Failed to find element")
            .expect("No element found");

        let element = parsed_doc
            .get_element(element_id)
            .expect("Failed to get element");

        if let Some(contents) = element.get_child_contents() {
            // Check for both comments and text
            let mut comment_count = 0;
            let mut text_found = false;

            for content in contents {
                match content {
                    XmlElementContentType::Comment(_) => comment_count += 1,
                    XmlElementContentType::Text(text) if text == "Some text content" => {
                        text_found = true
                    }
                    _ => {}
                }
            }

            assert_eq!(comment_count, 2, "Both comments should be preserved");
            assert!(text_found, "Text content should be preserved");
        } else {
            panic!("Element should have contents");
        }
    }

    #[test]
    fn test_element_positioning() {
        // Create a document with ordered elements
        let mut document = XmlDocument::new();

        // Create root element
        let root_id = document
            .create_root_element_mut("root", None)
            .expect("Failed to create root element");

        // Add elements in specific order
        document
            .append_child_element_mut(root_id, "first", None)
            .expect("Failed to append first element");

        document
            .append_child_element_mut(root_id, "middle", None)
            .expect("Failed to append middle element");

        document
            .append_child_element_mut(root_id, "last", None)
            .expect("Failed to append last element");

        // Insert before first element
        document
            .inser_child_element_before_first_tag_mut(root_id, "before-first", "first", None)
            .expect("Failed to insert before first element");

        // Insert after last element
        document
            .inser_child_element_after_last_tag_mut(root_id, "after-last", "last", None)
            .expect("Failed to insert after last element");

        // Insert after middle element
        document
            .inser_child_element_after_last_tag_mut(root_id, "after-middle", "middle", None)
            .expect("Failed to insert after middle element");

        // Verify order
        let root = document
            .get_element(root_id)
            .expect("Failed to get root element");

        if let Some(contents) = root.get_child_contents() {
            let mut element_tags = Vec::new();

            for content in contents {
                if let XmlElementContentType::Element((_, tag, _)) = content {
                    element_tags.push(tag);
                }
            }

            assert_eq!(
                element_tags,
                vec![
                    "before-first",
                    "first",
                    "middle",
                    "after-middle",
                    "last",
                    "after-last"
                ],
                "Elements should be in correct order"
            );
        } else {
            panic!("Root should have contents");
        }
    }

    #[test]
    fn test_namespace_handling() {
        // Create a document with namespaced elements
        let mut document = XmlDocument::new();

        // Create root element with namespace declaration
        let root_id = document
            .create_root_element_mut(
                "ns:root",
                Some(vec![XmlAttribute::new(
                    "xmlns:ns".to_string(),
                    "http://example.org/ns".to_string(),
                )]),
            )
            .expect("Failed to create root element");

        // Add namespaced elements
        document
            .append_child_element_mut(root_id, "ns:child", None)
            .expect("Failed to append child element");

        // Add element with a different namespace
        document
            .append_child_element_mut(
                root_id,
                "ns2:other",
                Some(vec![XmlAttribute::new(
                    "xmlns:ns2".to_string(),
                    "http://example.org/ns2".to_string(),
                )]),
            )
            .expect("Failed to append other element");

        // Insert namespaced element after ns:child
        document
            .inser_child_element_after_last_tag_ns_mut(root_id, "ns:sibling", "ns:child", None)
            .expect("Failed to insert after namespaced element");

        // Serialize and verify
        let xml_bytes = XmlSerializer::xml_tree_to_vec(&mut document)
            .expect("Failed to serialize document with namespaces");

        // The serialized XML should contain the namespace declarations
        let xml_string = String::from_utf8(xml_bytes.clone()).unwrap();
        assert!(
            xml_string.contains("xmlns:ns="),
            "Namespace declaration should be preserved"
        );
        assert!(
            xml_string.contains("xmlns:ns2="),
            "Second namespace declaration should be preserved"
        );

        // Parse it back
        let parsed_doc = XmlDeserializer::vec_to_xml_doc_tree(xml_bytes)
            .expect("Failed to parse document with namespaces");

        // Check structure using namespaced tag searches
        let parsed_root_id = parsed_doc.get_root_id();

        // Find child by namespaced tag
        let ns_child_id = parsed_doc
            .find_first_child_ns(parsed_root_id, "ns:child")
            .expect("Failed to find namespaced child")
            .expect("No namespaced child found");

        let ns_child = parsed_doc
            .get_element(ns_child_id)
            .expect("Failed to get namespaced child");

        assert_eq!(
            ns_child.get_tag(),
            "child",
            "Local tag name should be 'child'"
        );
        assert_eq!(
            ns_child.get_tag_ns(),
            "ns:child",
            "Namespaced tag should be 'ns:child'"
        );
    }

    #[test]
    fn test_clear_element_content() {
        // Create a document with nested elements
        let mut document = XmlDocument::new();

        // Create root element
        let root_id = document
            .create_root_element_mut("root", None)
            .expect("Failed to create root element");

        // Add a parent element
        let parent_id = document
            .append_child_element_mut(root_id, "parent", None)
            .expect("Failed to append parent element");

        // Add children to the parent
        document
            .append_child_element_mut(parent_id, "child1", None)
            .expect("Failed to append child1");

        document
            .append_child_element_mut(parent_id, "child2", None)
            .expect("Failed to append child2");

        // Verify structure before clearing
        let parent_before = document
            .get_element(parent_id)
            .expect("Failed to get parent element");

        if let Some(contents) = parent_before.get_child_contents() {
            assert_eq!(
                contents.len(),
                2,
                "Parent should have 2 children before clearing"
            );
        } else {
            panic!("Parent should have contents before clearing");
        }

        // Clear the parent's content
        document
            .clear_element_content_mut(parent_id)
            .expect("Failed to clear parent content");

        // Verify parent is now empty
        let parent_after = document
            .get_element(parent_id)
            .expect("Failed to get parent element after clearing");

        assert!(
            parent_after.get_child_contents().is_none(),
            "Parent should have no contents after clearing"
        );
    }

    #[test]
    fn test_find_all_by_attribute_ns() {
        let xml_input = r#"
        <root xmlns:test="http://example.org/test">
            <item test:type="important" id="1">Item 1</item>
            <item id="2">Item 2</item>
            <item test:type="important" id="3">Item 3</item>
            <item test:type="normal" id="4">Item 4</item>
        </root>
        "#;

        // Parse the XML
        let document = XmlDeserializer::vec_to_xml_doc_tree(xml_input.as_bytes().to_vec())
            .expect("Failed to deserialize XML");

        // Get the root element
        let root_id = document.get_root_id();

        // Find items with test:type="important"
        let important_items = document
            .find_all_by_attribute_ns(root_id, "test:type", "important")
            .expect("Failed to search for attribute")
            .expect("No items with test:type='important' found");

        assert_eq!(important_items.len(), 2, "Should find 2 important items");

        // Verify they're the correct elements
        for item_id in important_items {
            let item = document
                .get_element(item_id)
                .expect("Failed to get item element");

            let id_attr = item
                .get_attribute("id")
                .expect("Item should have id attribute");
            assert!(
                id_attr.get_value() == "1" || id_attr.get_value() == "3",
                "Important items should have id 1 or 3"
            );
        }
    }

    #[test]
    fn test_xml_declaration() {
        // Create a simple document
        let mut document = XmlDocument::new();

        // Set custom version and encoding
        document.set_version_mut("1.1".to_string());
        document.set_encoding_mut("ISO-8859-1".to_string());

        // Add a root element
        document
            .create_root_element_mut("root", None)
            .expect("Failed to create root element");

        // Serialize the document
        let xml_bytes =
            XmlSerializer::xml_tree_to_vec(&mut document).expect("Failed to serialize document");

        // Check the XML declaration in the output
        let xml_string = String::from_utf8(xml_bytes).unwrap();

        // In debug mode, it should use the custom values
        #[cfg(debug_assertions)]
        {
            assert!(
                xml_string.starts_with("<?xml version=\"1.1\" encoding=\"ISO-8859-1\"?>"),
                "XML declaration should use custom version and encoding"
            );
        }

        // In release mode, it uses fixed values
        #[cfg(not(debug_assertions))]
        {
            assert!(
                xml_string.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"),
                "XML declaration should use fixed values in release mode"
            );
        }
    }

    #[test]
    fn test_xml_no_alias_ns_tag() {
        let mut doc = XmlDocument::new();
        let root_id = doc
            .create_root_element_mut("test", None)
            .expect("Failed to create root element");
        doc.append_child_element_mut(root_id, "newtag", None)
            .expect("Failed to create child element");
        assert!(doc
            .append_child_element_mut(root_id, "a:invalidns", None)
            .is_err())
    }

    #[test]
    fn test_xml_no_alias_ns_attribute() {
        let mut doc = XmlDocument::new();
        let root_id = doc
            .create_root_element_mut("test", None)
            .expect("Failed to create root element");
        doc.append_child_element_mut(root_id, "newtag", None)
            .expect("Failed to create child element");
        assert!(doc
            .append_child_element_mut(
                root_id,
                "another",
                Some(vec![XmlAttribute::new(
                    "ns:invalid".to_string(),
                    "value".to_string()
                )])
            )
            .is_err())
    }

    #[test]
    fn test_xml_attribute_ns_name_and_keys() {
        let xml_attribute = XmlAttribute::new("ns:attr".to_string(), "val".to_string());
        assert_eq!(xml_attribute.get_name(), "attr");
        assert_eq!(xml_attribute.get_ns_name(), "ns:attr");
        assert_eq!(xml_attribute.get_value(), "val");

        let mut document = XmlDocument::new();
        let root_id = document.create_root_element_mut("root", None).unwrap();
        let child_element_id = document
            .append_child_element_mut(
                root_id,
                "child",
                Some(vec![
                    XmlAttribute::new("id".to_string(), "1".to_string()),
                    XmlAttribute::new("xmlns:ns".to_string(), "http://example.org/ns".to_string()),
                    XmlAttribute::new("ns:kind".to_string(), "special".to_string()),
                ]),
            )
            .expect("append child");

        let child_element = document.get_element(child_element_id).expect("get element");
        let attribute_keys = child_element.get_attribute_keys().expect("keys");
        assert!(attribute_keys.contains(&"id".to_string()));
        let ns_attribute_keys = child_element.get_attribute_ns_keys().expect("ns keys");
        assert!(ns_attribute_keys.contains(&"ns:kind".to_string()));
    }

    #[test]
    fn test_element_attribute_mutations_and_counts() {
        let mut document = XmlDocument::new();
        let root_id = document.create_root_element_mut("root", None).unwrap();
        let item_element_id = document
            .append_child_element_mut(
                root_id,
                "item",
                Some(vec![XmlAttribute::new("a".to_string(), "1".to_string())]),
            )
            .unwrap();

        {
            let element_mut = document.get_element_mut(item_element_id).unwrap();
            element_mut
                .add_replace_attribute_mut(XmlAttribute::new("a".to_string(), "2".to_string()))
                .expect("replace attr");
        }

        let item_element = document.get_element(item_element_id).unwrap();
        assert_eq!(item_element.get_attribute("a").unwrap().get_value(), "2");

        {
            let element_mut = document.get_element_mut(item_element_id).unwrap();
            let res = element_mut
                .set_attribute_mut(vec![XmlAttribute::new("x".to_string(), "y".to_string())]);
            assert!(res.is_err());
        }

        {
            let element_mut = document.get_element_mut(item_element_id).unwrap();
            let removed_count = element_mut.clear_attribute_mut().expect("clear attrs");
            assert!(removed_count >= 1);
            assert!(element_mut.get_attribute_keys().is_none());
        }
    }

    #[test]
    fn test_remove_attribute_ns_and_children_count_text() {
        let mut document = XmlDocument::new();
        let root_id = document.create_root_element_mut("root", None).unwrap();
        let item_element_id = document
            .append_child_element_mut(
                root_id,
                "item",
                Some(vec![
                    XmlAttribute::new("xmlns:ns".to_string(), "http://example.org/ns".to_string()),
                    XmlAttribute::new("ns:tag".to_string(), "v".to_string()),
                ]),
            )
            .unwrap();

        {
            let element_mut = document.get_element_mut(item_element_id).unwrap();
            element_mut.remove_attribute_ns_mut("ns:tag");
        }

        let item_element = document.get_element(item_element_id).unwrap();
        assert!(item_element.get_attribute_ns("ns:tag").is_none());

        {
            let element_mut = document.get_element_mut(item_element_id).unwrap();
            element_mut.add_text_mut("hello").unwrap();
            document
                .append_child_element_mut(item_element_id, "dummy", None)
                .expect("append child");
        }

        let item_element = document.get_element(item_element_id).unwrap();
        let text_value = item_element.get_element_text_value().unwrap();
        assert_eq!(text_value.unwrap(), "hello");
        let child_count = item_element.get_child_element_count().unwrap();
        assert!(child_count >= 1);
    }

    #[test]
    fn test_clone_document_and_query_and_serialization_file_roundtrip() {
        let mut document = XmlDocument::new();
        let root_id = document.create_root_element_mut("root", None).unwrap();
        let _ = document
            .append_child_element_mut(
                root_id,
                "c",
                Some(vec![XmlAttribute::new("k".to_string(), "v".to_string())]),
            )
            .unwrap();

        let cloned_document = document.clone();
        assert_eq!(cloned_document.get_root_id(), document.get_root_id());

        let query_result = document.query_xpath("/root").unwrap();
        assert!(query_result.is_none());

        let mut tmp_file_path = std::env::temp_dir();
        tmp_file_path.push("xml_rs_test_roundtrip.xml");
        let tmp_file_path_str = tmp_file_path.to_str().unwrap().to_string();

        XmlSerializer::xml_doc_tree_to_file(&mut document, &tmp_file_path_str).expect("write file");
        let parsed_document =
            XmlDeserializer::file_to_xml_doc_tree(&tmp_file_path_str).expect("read file");
        assert!(parsed_document.get_root_id() > 0);

        let _ = std::fs::remove_file(&tmp_file_path_str);
    }

    // --- URI-based namespace lookup tests ---

    #[test]
    fn test_get_attribute_by_uri_standard_alias() {
        // r:embed with xmlns:r on the same element — canonical OOXML blip pattern
        let xml = r#"<root><a:blip xmlns:a="http://drawingml" xmlns:r="http://relationships" r:embed="rId5"/></root>"#;
        let doc =
            XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec()).expect("parse failed");
        let root_id = doc.get_root_id();
        let blip_id = doc
            .get_element(root_id)
            .unwrap()
            .find_first_child_ns("a:blip")
            .expect("blip not found");
        let blip = doc.get_element(blip_id).unwrap();
        let attr = blip
            .get_attribute_by_uri("http://relationships", "embed")
            .expect("should find r:embed by URI");
        assert_eq!(attr.get_value(), "rId5");
    }

    #[test]
    fn test_get_attribute_by_uri_non_standard_alias() {
        // Same semantics, but the file uses rel: instead of r:
        let xml = r#"<root><a:blip xmlns:a="http://drawingml" xmlns:rel="http://relationships" rel:embed="rId7"/></root>"#;
        let doc =
            XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec()).expect("parse failed");
        let root_id = doc.get_root_id();
        let blip_id = doc
            .get_element(root_id)
            .unwrap()
            .find_first_child_ns("a:blip")
            .expect("blip not found");
        let blip = doc.get_element(blip_id).unwrap();
        let attr = blip
            .get_attribute_by_uri("http://relationships", "embed")
            .expect("should find rel:embed by URI regardless of alias");
        assert_eq!(attr.get_value(), "rId7");
    }

    #[test]
    fn test_get_attribute_by_uri_inherited_alias() {
        // xmlns:r declared on parent; child uses r:embed without re-declaring it
        let xml = r#"<root xmlns:r="http://relationships"><child r:id="rId3"/></root>"#;
        let doc = XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec())
            .expect("parse failed — namespace inheritance broken");
        let root_id = doc.get_root_id();
        let child_id = doc
            .get_element(root_id)
            .unwrap()
            .find_first_child("child")
            .expect("child not found");
        let child = doc.get_element(child_id).unwrap();
        let attr = child
            .get_attribute_by_uri("http://relationships", "id")
            .expect("should find r:id via inherited namespace");
        assert_eq!(attr.get_value(), "rId3");
    }

    #[test]
    fn test_get_attribute_by_uri_inherited_after_sibling_override() {
        // Child B declares xmlns:a but should still inherit xmlns:r from parent
        let xml = r#"<root xmlns:r="http://relationships"><blip xmlns:a="http://drawingml" r:embed="rId9"/></root>"#;
        let doc = XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec())
            .expect("parse failed — inheritance broken after sibling override");
        let root_id = doc.get_root_id();
        let blip_id = doc
            .get_element(root_id)
            .unwrap()
            .find_first_child("blip")
            .expect("blip not found");
        let blip = doc.get_element(blip_id).unwrap();
        let attr = blip
            .get_attribute_by_uri("http://relationships", "embed")
            .expect("should find r:embed after inheriting r from parent");
        assert_eq!(attr.get_value(), "rId9");
    }

    #[test]
    fn test_get_alias_for_uri_returns_in_scope_alias() {
        let xml = r#"<root xmlns:r="http://relationships"><child r:id="rId1"/></root>"#;
        let doc = XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec()).unwrap();
        let root_id = doc.get_root_id();
        let child_id = doc
            .get_element(root_id)
            .unwrap()
            .find_first_child("child")
            .unwrap();
        let child = doc.get_element(child_id).unwrap();
        let alias = child
            .get_alias_for_uri("http://relationships")
            .expect("alias should be in scope via inheritance");
        assert_eq!(alias, "r");
    }

    // --- NamespaceDeclaration driven API tests ---

    const DRAWINGML_NS: NamespaceDeclaration = NamespaceDeclaration::new("http://drawingml", "a");
    const RELATIONSHIPS_NS: NamespaceDeclaration =
        NamespaceDeclaration::new("http://relationships", "r");

    #[test]
    fn test_ns_decl_root_emits_declaration() {
        let mut document = XmlDocument::new();
        document
            .create_root_element_ns_mut(&DRAWINGML_NS, "wsDr", None)
            .expect("failed to create ns root");
        let xml = String::from_utf8(
            XmlSerializer::xml_tree_to_vec(&mut document).expect("serialize failed"),
        )
        .unwrap();
        assert!(xml.contains("<a:wsDr"), "root should use default alias");
        assert!(
            xml.contains("xmlns:a=\"http://drawingml\""),
            "root should declare the namespace"
        );
    }

    #[test]
    fn test_ns_decl_child_reuses_in_scope_alias() {
        let mut document = XmlDocument::new();
        let root_id = document
            .create_root_element_ns_mut(&DRAWINGML_NS, "wsDr", None)
            .unwrap();
        document
            .append_child_element_ns_mut(root_id, &DRAWINGML_NS, "blip", None)
            .expect("failed to append ns child");
        let xml = String::from_utf8(
            XmlSerializer::xml_tree_to_vec(&mut document).expect("serialize failed"),
        )
        .unwrap();
        assert!(xml.contains("<a:blip"), "child should reuse in-scope alias");
        assert_eq!(
            xml.matches("xmlns:a=").count(),
            1,
            "namespace must be declared only once at the root"
        );
    }

    #[test]
    fn test_ns_decl_child_auto_declares_missing_namespace() {
        let mut document = XmlDocument::new();
        let root_id = document
            .create_root_element_ns_mut(&DRAWINGML_NS, "wsDr", None)
            .unwrap();
        document
            .append_child_element_ns_mut(root_id, &RELATIONSHIPS_NS, "child", None)
            .expect("failed to append child with new namespace");
        let xml = String::from_utf8(
            XmlSerializer::xml_tree_to_vec(&mut document).expect("serialize failed"),
        )
        .unwrap();
        assert!(xml.contains("<r:child"), "child should use default alias");
        assert!(
            xml.contains("xmlns:r=\"http://relationships\""),
            "missing namespace should be declared on the child"
        );
    }

    #[test]
    fn test_ns_decl_override_forces_alias() {
        let overridden = NamespaceDeclaration::with_override("http://drawingml", "a", "draw");
        let mut document = XmlDocument::new();
        let root_id = document
            .create_root_element_ns_mut(&DRAWINGML_NS, "wsDr", None)
            .unwrap();
        let alias = document
            .resolve_alias_mut(root_id, &overridden)
            .expect("resolve failed");
        assert_eq!(alias, "draw", "override must win over the in-scope alias");
    }

    #[test]
    fn test_ns_decl_add_attribute_reuses_and_declares() {
        let mut document = XmlDocument::new();
        let root_id = document
            .create_root_element_ns_mut(&DRAWINGML_NS, "wsDr", None)
            .unwrap();
        let blip_id = document
            .append_child_element_ns_mut(root_id, &DRAWINGML_NS, "blip", None)
            .unwrap();
        // Relationships namespace is not in scope; adding the attribute must declare it here.
        document
            .get_element_mut(blip_id)
            .unwrap()
            .add_attribute_ns_mut(&RELATIONSHIPS_NS, "embed", "rId5")
            .expect("failed to add namespaced attribute");

        let found = document
            .get_element(blip_id)
            .unwrap()
            .get_attribute_by_ns(&RELATIONSHIPS_NS, "embed")
            .expect("attribute should be retrievable by namespace");
        assert_eq!(found.get_value(), "rId5");

        let xml = String::from_utf8(
            XmlSerializer::xml_tree_to_vec(&mut document).expect("serialize failed"),
        )
        .unwrap();
        assert!(
            xml.contains("r:embed=\"rId5\""),
            "attribute should serialize with alias"
        );
        assert!(
            xml.contains("xmlns:r=\"http://relationships\""),
            "attribute namespace should be declared on the blip element"
        );
    }

    #[test]
    fn test_ns_decl_resolve_alias_read_only() {
        let mut document = XmlDocument::new();
        let root_id = document
            .create_root_element_ns_mut(&DRAWINGML_NS, "wsDr", None)
            .unwrap();
        assert_eq!(document.resolve_alias(root_id, &DRAWINGML_NS).unwrap(), "a");
        assert_eq!(
            document.resolve_alias(root_id, &RELATIONSHIPS_NS).unwrap(),
            "r"
        );
    }

    #[test]
    fn test_standalone_declaration_preserved() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><root/>"#;
        let document = XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec())
            .expect("parse failed");
        assert_eq!(document.get_standalone(), Some("yes"));
        let output = String::from_utf8(
            XmlSerializer::xml_tree_to_vec(&document).expect("serialize failed"),
        )
        .unwrap();
        assert!(output.contains("standalone=\"yes\""));
    }

    #[test]
    fn test_prolog_comments_preserved() {
        let xml = r#"<!-- first --><!-- second --><root/>"#;
        let document = XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec())
            .expect("parse failed");
        assert_eq!(document.get_prolog_comments().len(), 2);
        let output = String::from_utf8(
            XmlSerializer::xml_tree_to_vec(&document).expect("serialize failed"),
        )
        .unwrap();
        assert!(output.contains("<!-- first -->"));
        assert!(output.contains("<!-- second -->"));
    }

    #[test]
    fn test_default_serialization_preserves_unused_namespace() {
        let xml = r#"<root xmlns:used="http://used" xmlns:unused="http://unused"><used:child/></root>"#;
        let document = XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec())
            .expect("parse failed");
        let output = String::from_utf8(
            XmlSerializer::xml_tree_to_vec(&document).expect("serialize failed"),
        )
        .unwrap();
        assert!(output.contains("xmlns:unused=\"http://unused\""));
        assert!(output.contains("xmlns:used=\"http://used\""));
    }

    #[test]
    fn test_optimize_drops_unused_namespace() {
        let xml = r#"<root xmlns:used="http://used" xmlns:unused="http://unused"><used:child/></root>"#;
        let document = XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec())
            .expect("parse failed");
        let optimized = String::from_utf8(
            XmlSerializer::xml_tree_to_vec_with(
                &document,
                &SerializeOptions {
                    optimize_namespaces: true,
                },
            )
            .expect("serialize failed"),
        )
        .unwrap();
        assert!(!optimized.contains("http://unused"), "unused namespace should be dropped");
        assert!(optimized.contains("xmlns:used=\"http://used\""), "used namespace should remain");
    }

    #[test]
    fn test_optimize_hoists_redeclared_namespace_to_common_ancestor() {
        let xml = r#"<workbook xmlns="http://main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="x15" xmlns:x15="http://x15"><extLst><ext xmlns:x15="http://x15"><x15:workbookPr/></ext></extLst></workbook>"#;
        let document = XmlDeserializer::vec_to_xml_doc_tree(xml.as_bytes().to_vec())
            .expect("parse failed");
        let optimized = String::from_utf8(
            XmlSerializer::xml_tree_to_vec_with(
                &document,
                &SerializeOptions {
                    optimize_namespaces: true,
                },
            )
            .expect("serialize failed"),
        )
        .unwrap();
        assert!(optimized.contains("xmlns:x15=\"http://x15\""));
        assert_eq!(
            optimized.matches("xmlns:x15=").count(),
            1,
            "the redeclared namespace should collapse to a single declaration"
        );
    }
}
