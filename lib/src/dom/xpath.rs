use crate::Tag;

#[derive(Debug)]
enum PathType {
    Absolute,
    Relative,
    Anywhere,
}

#[derive(Debug)]
struct XPathPart{
    tag:Tag,
    
}

#[derive(Debug)]
pub(crate) struct XPathHandler {
    primary_query: String,
    path_type: PathType,
    path_group: Vec<String>,
}

impl XPathHandler {
    pub(crate) fn new(query: &str) -> XPathHandler {
        let path_type = if query.starts_with("//") {
            PathType::Anywhere
        } else if query.starts_with("/") {
            PathType::Relative
        } else {
            PathType::Absolute
        };
        Self {
            primary_query: query.to_owned(),
            path_type,
            path_group: query.split("/").map(|path| path.to_owned()).collect(),
        }
    }
}
