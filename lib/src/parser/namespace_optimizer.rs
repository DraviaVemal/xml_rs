/*
 * This file is part of the xml_rs project.
 * License: AGPL-3.0 (see ./LICENSE for details).
 * - Free for non-commercial/open source use under AGPL-3.0.
 * - Commercial use requires a separate license.
 */

use crate::{NodeId, XmlDocument, XmlElementContentType};
use anyhow::{Context, Error as AnyError, Result as AnyResult};
use std::collections::HashMap;
use super::xml_serializer::NamespacePlan;

const MARKUP_COMPATIBILITY_URI: &str =
    "http://schemas.openxmlformats.org/markup-compatibility/2006";

/// Computes an optimized placement for prefixed namespace declarations.
///
/// The optimizer hoists every used `alias -> uri` binding to the lowest common ancestor of the
/// nodes that reference it and drops bindings that are never referenced. The default namespace
/// is intentionally left untouched and is emitted from each element's stored declarations.
pub(crate) struct NamespaceOptimizer;

impl NamespaceOptimizer {
    /// Builds a plan mapping each node to the prefixed namespace declarations it must emit.
    pub(crate) fn build_plan(document: &XmlDocument) -> AnyResult<NamespacePlan, AnyError> {
        let mut parent_of: HashMap<NodeId, Option<NodeId>> = HashMap::new();
        let mut depth_of: HashMap<NodeId, u32> = HashMap::new();
        let mut ordered_nodes: Vec<NodeId> = Vec::new();

        let root_id = document.get_root_id();
        let mut traversal_stack = vec![(root_id, None::<NodeId>, 0u32)];
        while let Some((node_id, parent_id, depth)) = traversal_stack.pop() {
            parent_of.insert(node_id, parent_id);
            depth_of.insert(node_id, depth);
            ordered_nodes.push(node_id);
            let element = document
                .get_element(node_id)
                .context("draviavemal-xml_rs::Optimizer failed to read element")?;
            if let Some(contents) = element.get_child_contents() {
                for content in contents {
                    if let XmlElementContentType::Element((child_id, _, _)) = content {
                        traversal_stack.push((*child_id, Some(node_id), depth + 1));
                    }
                }
            }
        }

        let mut usage_sites: HashMap<(String, String), Vec<NodeId>> = HashMap::new();
        for node_id in &ordered_nodes {
            let element = document
                .get_element(*node_id)
                .context("draviavemal-xml_rs::Optimizer failed to read element")?;

            if let Some(alias) = element.get_tag_alias() {
                if !alias.is_empty() {
                    if let Some(uri) = element.resolve_alias_to_uri(alias) {
                        usage_sites
                            .entry((alias.to_owned(), uri))
                            .or_default()
                            .push(*node_id);
                    }
                }
            }

            if let Some(attributes) = element.get_attributes() {
                for attribute in attributes {
                    let alias = match attribute.get_ns_alias() {
                        Some(alias) if !alias.is_empty() => alias,
                        _ => continue,
                    };
                    let resolved_uri = element.resolve_alias_to_uri(alias);
                    if let Some(uri) = resolved_uri.clone() {
                        usage_sites
                            .entry((alias.to_owned(), uri))
                            .or_default()
                            .push(*node_id);
                    }
                    if resolved_uri.as_deref() == Some(MARKUP_COMPATIBILITY_URI) {
                        for token in attribute.get_value().split_whitespace() {
                            if let Some(token_uri) = element.resolve_alias_to_uri(token) {
                                usage_sites
                                    .entry((token.to_owned(), token_uri))
                                    .or_default()
                                    .push(*node_id);
                            }
                        }
                    }
                }
            }
        }

        let mut sorted_keys: Vec<(String, String)> = usage_sites.keys().cloned().collect();
        sorted_keys.sort();

        let mut declared_at: HashMap<NodeId, HashMap<String, String>> = HashMap::new();
        for key in sorted_keys {
            let (alias, uri) = key.clone();
            let sites = &usage_sites[&key];
            let anchor = Self::lowest_common_ancestor(&parent_of, &depth_of, sites);
            let target_nodes = match anchor {
                Some(anchor_id) if Self::alias_is_free(&declared_at, anchor_id, &alias, &uri) => {
                    vec![anchor_id]
                }
                _ => sites.clone(),
            };
            for target in target_nodes {
                declared_at
                    .entry(target)
                    .or_default()
                    .insert(alias.clone(), uri.clone());
            }
        }

        let mut plan: NamespacePlan = HashMap::new();
        for (node_id, declarations) in declared_at {
            let mut entries: Vec<(String, String)> = declarations.into_iter().collect();
            entries.sort();
            plan.insert(node_id, entries);
        }
        Ok(plan)
    }

    fn alias_is_free(
        declared_at: &HashMap<NodeId, HashMap<String, String>>,
        node_id: NodeId,
        alias: &str,
        uri: &str,
    ) -> bool {
        match declared_at.get(&node_id).and_then(|bindings| bindings.get(alias)) {
            Some(existing_uri) => existing_uri == uri,
            None => true,
        }
    }

    fn lowest_common_ancestor(
        parent_of: &HashMap<NodeId, Option<NodeId>>,
        depth_of: &HashMap<NodeId, u32>,
        nodes: &[NodeId],
    ) -> Option<NodeId> {
        let mut node_iterator = nodes.iter();
        let mut common = *node_iterator.next()?;
        for node in node_iterator {
            common = Self::pairwise_lowest_common_ancestor(parent_of, depth_of, common, *node)?;
        }
        Some(common)
    }

    fn pairwise_lowest_common_ancestor(
        parent_of: &HashMap<NodeId, Option<NodeId>>,
        depth_of: &HashMap<NodeId, u32>,
        first: NodeId,
        second: NodeId,
    ) -> Option<NodeId> {
        let mut higher = first;
        let mut lower = second;
        let mut higher_depth = *depth_of.get(&higher)?;
        let mut lower_depth = *depth_of.get(&lower)?;
        while higher_depth > lower_depth {
            higher = (*parent_of.get(&higher)?)?;
            higher_depth -= 1;
        }
        while lower_depth > higher_depth {
            lower = (*parent_of.get(&lower)?)?;
            lower_depth -= 1;
        }
        while higher != lower {
            higher = (*parent_of.get(&higher)?)?;
            lower = (*parent_of.get(&lower)?)?;
        }
        Some(higher)
    }
}
