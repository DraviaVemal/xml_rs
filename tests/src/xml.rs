#[cfg(test)]
mod xml_test {
    use draviavemal_xml_rs::{
        XmlAttribute, XmlDeserializer, XmlDocument, XmlElementContentType, XmlSerializer,
    };

    /// Test data for common XML test cases
    fn get_test_xml() -> &'static str {
        r#"
        <test:catalog xmlns:test="http://example.org/test">
            <!-- This is Test Content 1 -->
            <!-- This is Test Content 2 -->
            <book id="bk101">
                <author>John Doe</author>
                <title>XML Basics</title>
                <genre>Programming</genre>
                <price>29.95</price>
                <empty/>
                <newempty attr1="value" attr2="value2"/>
                <publish_date>
                    <date>01</date>
                    <month>01</month>
                    <year>2024</year>
                </publish_date>
                <description>An introduction to XML.</description>
            </book>
            <book id="bk102">
                <author>Jane Smith</author>
                <title>Advanced XML</title>
                <genre>Programming</genre>
                <price>39.95</price>
                <publish_date>2024-06-02</publish_date>
                <description>Deep dive into XML technologies.</description>
            </book>
        </test:catalog>
    "#
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

        // The root should be a "test:catalog" element
        assert_eq!(root.get_tag(), "catalog", "Root tag should be 'catalog'");

        // It should have 2 book children
        let book_ids = document
            .find_all_child(root_id, "book")
            .expect("Failed to find books")
            .expect("No book elements found");

        assert_eq!(book_ids.len(), 2, "Should have 2 book elements");

        // Get the first book
        let book1 = document
            .get_element(book_ids[0])
            .expect("Failed to get first book");

        // It should have an id attribute
        if let Some(attr) = book1.get_attribute("id") {
            assert_eq!(
                attr.get_value(),
                "bk101",
                "First book should have id='bk101'"
            );
        } else {
            panic!("First book should have attributes");
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

        // Find book with id="bk101"
        let book_id = document
            .find_first_by_attribute(root_id, "id", "bk101")
            .expect("Failed to search for attribute")
            .expect("No book with id='bk101' found");

        // Verify it's the right element
        document
            .get_element(book_id)
            .expect("Failed to get book element");

        // Find the title child
        let title_id = document
            .find_first_child(book_id, "title")
            .expect("Failed to find title")
            .expect("No title element found");

        let title = document
            .get_element(title_id)
            .expect("Failed to get title element");

        // Check the title's content
        if let Some(contents) = title.get_child_contents() {
            let has_correct_text = contents.iter().any(|content| {
                if let XmlElementContentType::Text(text) = content {
                    text == "XML Basics"
                } else {
                    false
                }
            });

            assert!(has_correct_text, "Title should contain 'XML Basics'");
        } else {
            panic!("Title should have content");
        }
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
}
