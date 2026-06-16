use petgraph::Direction;
use petgraph::algo::kosaraju_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Dfs, EdgeRef};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CycleReport {
    pub(crate) cycles: Vec<BTreeSet<String>>,
    pub(crate) cycle_nodes: BTreeSet<String>,
    pub(crate) dependent_nodes: BTreeSet<String>,
}

#[derive(Debug, Default)]
pub(crate) struct ReferenceGraph {
    graph: DiGraph<String, ()>,
    node_indices: HashMap<String, NodeIndex>,
}

impl ReferenceGraph {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn rebuild<'a>(
        &mut self,
        entries: impl IntoIterator<Item = (&'a str, &'a BTreeSet<String>)>,
    ) {
        self.graph = DiGraph::new();
        self.node_indices.clear();

        let entries = entries.into_iter().collect::<Vec<_>>();

        for (id, _) in &entries {
            let node = self.graph.add_node((*id).to_string());
            self.node_indices.insert((*id).to_string(), node);
        }

        for (id, references) in entries {
            let Some(&dependent) = self.node_indices.get(id) else {
                continue;
            };

            for reference in references {
                if let Some(&dependency) = self.node_indices.get(reference) {
                    self.graph.add_edge(dependency, dependent, ());
                }
            }
        }
    }

    pub(crate) fn affected_by(&self, id: &str) -> BTreeSet<String> {
        let mut affected = BTreeSet::from([id.to_string()]);
        let Some(&start) = self.node_indices.get(id) else {
            return affected;
        };

        affected.extend(self.dependents_from([start]));
        affected
    }

    pub(crate) fn cycle_report(&self) -> CycleReport {
        let mut report = CycleReport::default();

        for component in kosaraju_scc(&self.graph) {
            let is_cycle =
                component.len() > 1 || component.iter().any(|&node| self.has_self_reference(node));

            if !is_cycle {
                continue;
            }

            let cycle = component
                .iter()
                .map(|&node| self.graph[node].clone())
                .collect::<BTreeSet<_>>();
            report.cycle_nodes.extend(cycle.iter().cloned());
            report.cycles.push(cycle);
        }

        report.cycles.sort();
        report.dependent_nodes = self
            .dependents_of_ids(&report.cycle_nodes)
            .difference(&report.cycle_nodes)
            .cloned()
            .collect();

        report
    }

    pub(crate) fn evaluation_order(&self, ids: BTreeSet<String>) -> Vec<String> {
        let mut ordered = kosaraju_scc(&self.graph)
            .into_iter()
            .rev()
            .flat_map(|component| {
                let mut component_ids = component
                    .into_iter()
                    .map(|node| self.graph[node].clone())
                    .filter(|id| ids.contains(id))
                    .collect::<Vec<_>>();
                component_ids.sort();
                component_ids
            })
            .collect::<Vec<_>>();

        let present = ordered.iter().cloned().collect::<BTreeSet<_>>();
        ordered.extend(ids.into_iter().filter(|id| !present.contains(id)));
        ordered
    }

    fn has_self_reference(&self, node: NodeIndex) -> bool {
        self.graph
            .edges_directed(node, Direction::Outgoing)
            .any(|edge| edge.target() == node)
    }

    fn dependents_of_ids(&self, ids: &BTreeSet<String>) -> BTreeSet<String> {
        let starts = ids
            .iter()
            .filter_map(|id| self.node_indices.get(id).copied())
            .collect::<Vec<_>>();

        self.dependents_from(starts)
    }

    fn dependents_from<I>(&self, starts: I) -> BTreeSet<String>
    where
        I: IntoIterator<Item = NodeIndex>,
    {
        let mut dependents = BTreeSet::new();
        let mut dfs = Dfs::empty(&self.graph);

        for start in starts {
            dfs.move_to(start);

            while let Some(node) = dfs.next(&self.graph) {
                if node == start {
                    continue;
                }

                dependents.insert(self.graph[node].clone());
            }
        }

        dependents
    }
}
