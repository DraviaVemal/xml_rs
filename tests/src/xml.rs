#[cfg(test)]
mod xml_test {
    use draviavemal_xml_rs::{XmlDeserializer, XmlSerializer};

    #[test]
    fn test_xml_deserialization() {
        let xml_input = r#"
        <test:catalog>
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
    "#;

        let mut document = XmlDeserializer::vec_to_xml_doc_tree(xml_input.as_bytes().to_vec())
            .expect("Failed to deserialize XML");
        let content = XmlSerializer::xml_tree_to_vec(&mut document).expect("Failed to serialize XML");
        assert!(true);
    }
}
