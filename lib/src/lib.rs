/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

pub(crate) const XML_NS_DEFAULT: &str = "<Default>";

pub mod dom;
pub(crate) mod macros;
pub mod parser;
pub(crate) mod utils;

pub use dom::*;
pub use parser::*;
