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
}
