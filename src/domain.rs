use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceNode {
    pub name: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommunicationEdge {
    pub source: String,
    pub target: String,
    pub protocol: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyGraph {
    pub source: String,
    pub services: Vec<ServiceNode>,
    pub communications: Vec<CommunicationEdge>,
}

pub fn normalize_graph(mut graph: DependencyGraph) -> DependencyGraph {
    graph.services = dedupe_services(graph.services);
    graph.communications = dedupe_edges(graph.communications);
    graph
}

fn dedupe_services(services: Vec<ServiceNode>) -> Vec<ServiceNode> {
    services
        .into_iter()
        .fold(BTreeMap::<String, ServiceNode>::new(), |mut acc, svc| {
            acc.insert(svc.name.clone(), svc);
            acc
        })
        .into_values()
        .collect()
}

fn dedupe_edges(edges: Vec<CommunicationEdge>) -> Vec<CommunicationEdge> {
    edges
        .into_iter()
        .fold(
            BTreeMap::<(String, String, String), CommunicationEdge>::new(),
            |mut acc, edge| {
                acc.insert(
                    (
                        edge.source.clone(),
                        edge.target.clone(),
                        edge.protocol.clone(),
                    ),
                    edge,
                );
                acc
            },
        )
        .into_values()
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|edge| edge.source != edge.target)
        .collect::<Vec<_>>()
        .into_iter()
        .fold(BTreeSet::new(), |mut acc, edge| {
            acc.insert((edge.source, edge.target, edge.protocol, edge.metadata));
            acc
        })
        .into_iter()
        .map(|(source, target, protocol, metadata)| CommunicationEdge {
            source,
            target,
            protocol,
            metadata,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_graph_dedupes_and_drops_self_references() {
        let graph = DependencyGraph {
            source: "ci-adapter".to_string(),
            services: vec![
                ServiceNode {
                    name: "api".to_string(),
                    metadata: BTreeMap::new(),
                },
                ServiceNode {
                    name: "api".to_string(),
                    metadata: BTreeMap::new(),
                },
            ],
            communications: vec![
                CommunicationEdge {
                    source: "api".to_string(),
                    target: "worker".to_string(),
                    protocol: "http".to_string(),
                    metadata: BTreeMap::new(),
                },
                CommunicationEdge {
                    source: "api".to_string(),
                    target: "worker".to_string(),
                    protocol: "http".to_string(),
                    metadata: BTreeMap::new(),
                },
                CommunicationEdge {
                    source: "api".to_string(),
                    target: "api".to_string(),
                    protocol: "http".to_string(),
                    metadata: BTreeMap::new(),
                },
            ],
        };

        let normalized = normalize_graph(graph);
        assert_eq!(normalized.services.len(), 1);
        assert_eq!(normalized.communications.len(), 1);
    }
}
