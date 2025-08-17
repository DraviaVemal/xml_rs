/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

pub(crate) fn is_valid_xml_name(name: &str) -> bool {
    fn is_name_start_char(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_' || c.is_alphabetic()
    }
    fn is_name_char(c: char) -> bool {
        is_name_start_char(c) || c.is_ascii_digit() || c == '-' || c == '.' || c == ':'
    }
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    if let Some(first_char) = chars.next() {
        if !is_name_start_char(first_char) {
            return false;
        }
    }
    let mut colon_count = 0;
    for c in chars {
        if c == ':' {
            colon_count += 1;
            if colon_count > 1 {
                return false;
            }
        } else if !is_name_char(c) {
            return false;
        }
    }
    true
}
