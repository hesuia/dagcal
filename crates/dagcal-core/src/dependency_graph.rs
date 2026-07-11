use crate::id::ExpressionId;
use petgraph::Direction;
use petgraph::algo::kosaraju_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Dfs, EdgeRef};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CycleReport {
    pub(crate) cycles: Vec<BTreeSet<ExpressionId>>,
    pub(crate) cycle_nodes: BTreeSet<ExpressionId>,
    pub(crate) dependent_nodes: BTreeSet<ExpressionId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GraphAnalysis {
    pub(crate) cycle_report: CycleReport,
    pub(crate) evaluation_order: Vec<ExpressionId>,
}

#[derive(Debug, Default)]
pub(crate) struct ReferenceGraph {
    graph: DiGraph<ExpressionId, ()>,
    node_indices: HashMap<ExpressionId, NodeIndex>,
    references: HashMap<ExpressionId, BTreeSet<ExpressionId>>,
}

impl ReferenceGraph {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn rebuild(
        &mut self,
        entries: impl IntoIterator<Item = (ExpressionId, BTreeSet<ExpressionId>)>,
    ) {
        self.graph = DiGraph::new();
        self.node_indices.clear();
        self.references.clear();

        let entries = entries.into_iter().collect::<Vec<_>>();

        for (id, _) in &entries {
            let node = self.graph.add_node(*id);
            self.node_indices.insert(*id, node);
        }

        for (id, references) in entries {
            self.references.insert(id, references.clone());
            let Some(&dependent) = self.node_indices.get(&id) else {
                continue;
            };

            for reference in references {
                if let Some(&dependency) = self.node_indices.get(&reference) {
                    self.graph.add_edge(dependency, dependent, ());
                }
            }
        }
    }

    /// Inserts or replaces one entry's dependency edges without rebuilding the graph.
    pub(crate) fn upsert(&mut self, id: ExpressionId, references: BTreeSet<ExpressionId>) {
        let node = self.ensure_node(id);
        let incoming = self
            .graph
            .edges_directed(node, Direction::Incoming)
            .map(|edge| edge.id())
            .collect::<Vec<_>>();
        for edge in incoming {
            self.graph.remove_edge(edge);
        }

        self.references.insert(id, references.clone());
        for reference in references {
            if let Some(&dependency) = self.node_indices.get(&reference) {
                self.graph.add_edge(dependency, node, ());
            }
        }

        // A newly materialized ID can satisfy stored `$n` references.
        for (&dependent_id, dependent_references) in &self.references {
            if dependent_id != id && dependent_references.contains(&id) {
                let dependent = self.node_indices[&dependent_id];
                if self.graph.find_edge(node, dependent).is_none() {
                    self.graph.add_edge(node, dependent, ());
                }
            }
        }
    }

    /// Removes an entry node while retaining references to its stable ID.
    pub(crate) fn remove(&mut self, id: ExpressionId) {
        let Some(node) = self.node_indices.remove(&id) else {
            return;
        };
        self.references.remove(&id);
        self.graph.remove_node(node);
        self.node_indices.clear();
        for index in self.graph.node_indices() {
            self.node_indices.insert(self.graph[index], index);
        }
    }

    fn ensure_node(&mut self, id: ExpressionId) -> NodeIndex {
        if let Some(&node) = self.node_indices.get(&id) {
            return node;
        }
        let node = self.graph.add_node(id);
        self.node_indices.insert(id, node);
        node
    }

    pub(crate) fn affected_by(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        let mut affected = BTreeSet::from([id]);
        let Some(&start) = self.node_indices.get(&id) else {
            return affected;
        };

        affected.extend(self.dependents_from([start]));
        affected
    }

    pub(crate) fn dependencies_of(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        let Some(&node) = self.node_indices.get(&id) else {
            return BTreeSet::new();
        };

        self.graph
            .edges_directed(node, Direction::Incoming)
            .map(|edge| self.graph[edge.source()])
            .collect()
    }

    pub(crate) fn dependents_of(&self, id: ExpressionId) -> BTreeSet<ExpressionId> {
        let Some(&node) = self.node_indices.get(&id) else {
            return BTreeSet::new();
        };

        self.dependents_from([node])
    }

    pub(crate) fn cycle_report(&self) -> CycleReport {
        let components = kosaraju_scc(&self.graph);
        self.cycle_report_from_components(&components)
    }

    pub(crate) fn analyze(&self, ids: &BTreeSet<ExpressionId>) -> GraphAnalysis {
        let components = kosaraju_scc(&self.graph);

        GraphAnalysis {
            cycle_report: self.cycle_report_from_components(&components),
            evaluation_order: self.evaluation_order_from_components(&components, ids),
        }
    }

    fn cycle_report_from_components(&self, components: &[Vec<NodeIndex>]) -> CycleReport {
        let mut report = CycleReport::default();

        for component in components {
            let is_cycle =
                component.len() > 1 || component.iter().any(|&node| self.has_self_reference(node));

            if !is_cycle {
                continue;
            }

            let cycle = component
                .iter()
                .map(|&node| self.graph[node])
                .collect::<BTreeSet<_>>();
            report.cycle_nodes.extend(cycle.iter().copied());
            report.cycles.push(cycle);
        }

        report.cycles.sort();
        report.dependent_nodes = self
            .dependents_of_ids(&report.cycle_nodes)
            .difference(&report.cycle_nodes)
            .copied()
            .collect();

        report
    }

    fn evaluation_order_from_components(
        &self,
        components: &[Vec<NodeIndex>],
        ids: &BTreeSet<ExpressionId>,
    ) -> Vec<ExpressionId> {
        let mut ordered = components
            .iter()
            .rev()
            .flat_map(|component| {
                let mut component_ids = component
                    .iter()
                    .map(|node| self.graph[*node])
                    .filter(|id| ids.contains(id))
                    .collect::<Vec<_>>();
                component_ids.sort();
                component_ids
            })
            .collect::<Vec<_>>();

        let present = ordered.iter().copied().collect::<BTreeSet<_>>();
        ordered.extend(ids.iter().copied().filter(|id| !present.contains(id)));
        ordered
    }

    fn has_self_reference(&self, node: NodeIndex) -> bool {
        self.graph
            .edges_directed(node, Direction::Outgoing)
            .any(|edge| edge.target() == node)
    }

    fn dependents_of_ids(&self, ids: &BTreeSet<ExpressionId>) -> BTreeSet<ExpressionId> {
        let starts = ids
            .iter()
            .filter_map(|id| self.node_indices.get(id).copied());
        self.dependents_from(starts)
    }

    fn dependents_from<I>(&self, starts: I) -> BTreeSet<ExpressionId>
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

                dependents.insert(self.graph[node]);
            }
        }

        dependents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: usize) -> ExpressionId {
        ExpressionId::new(value)
    }

    #[test]
    fn upsert_replaces_only_the_changed_nodes_dependencies() {
        let mut graph = ReferenceGraph::new();
        graph.upsert(id(1), BTreeSet::new());
        graph.upsert(id(2), BTreeSet::from([id(1)]));
        graph.upsert(id(3), BTreeSet::from([id(2)]));

        graph.upsert(id(2), BTreeSet::new());

        assert_eq!(graph.affected_by(id(1)), BTreeSet::from([id(1)]));
        assert_eq!(graph.affected_by(id(2)), BTreeSet::from([id(2), id(3)]));
    }

    #[test]
    fn adding_a_missing_id_connects_existing_stable_id_references() {
        let mut graph = ReferenceGraph::new();
        graph.upsert(id(2), BTreeSet::from([id(1)]));
        assert_eq!(graph.affected_by(id(1)), BTreeSet::from([id(1)]));

        graph.upsert(id(1), BTreeSet::new());

        assert_eq!(graph.affected_by(id(1)), BTreeSet::from([id(1), id(2)]));
    }

    #[test]
    fn removal_keeps_other_node_indices_valid() {
        let mut graph = ReferenceGraph::new();
        graph.upsert(id(1), BTreeSet::new());
        graph.upsert(id(2), BTreeSet::new());
        graph.upsert(id(3), BTreeSet::from([id(2)]));

        graph.remove(id(1));

        assert_eq!(graph.affected_by(id(2)), BTreeSet::from([id(2), id(3)]));
    }
}
