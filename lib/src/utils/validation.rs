/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

/// Validates that a string is a valid XML name according to XML naming rules.
///
/// This function checks if the provided string conforms to XML 1.0 naming conventions:
/// - Must start with a letter, underscore, or other allowed starting character
/// - Can contain letters, digits, hyphens, periods, and colons
/// - Can have at most one colon (for namespace separation)
///
/// # Arguments
/// * `name` - The string to validate as an XML name.
///
/// # Returns
/// * `bool` - True if the name is valid, false otherwise.
pub(crate) fn is_valid_xml_name(name: &str) -> bool {
    // Helper function to check if a character is valid as the first character in an XML name
    fn is_name_start_char(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_' || c.is_alphabetic()
    }
    
    // Helper function to check if a character is valid anywhere in an XML name
    fn is_name_char(c: char) -> bool {
        is_name_start_char(c) || c.is_ascii_digit() || c == '-' || c == '.' || c == ':'
    }
    
    // Empty names are invalid
    if name.is_empty() {
        return false;
    }
    
    // Check the first character
    let mut chars = name.chars();
    if let Some(first_char) = chars.next() {
        if !is_name_start_char(first_char) {
            return false;
        }
    }
    
    // Check remaining characters and count colons
    let mut colon_count = 0;
    for c in chars {
        if c == ':' {
            colon_count += 1;
            // XML allows at most one colon (for namespace separation)
            if colon_count > 1 {
                return false;
            }
        } else if !is_name_char(c) {
            return false;
        }
    }
    
    true
}
