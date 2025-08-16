pub(crate) const XML_NS_DEFAULT: &str = "<Default>";

pub(crate) mod macros;
pub mod parser;
pub(crate) mod utils;
pub mod dom;

pub use parser::*;
pub use dom::*;
