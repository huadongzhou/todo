//! Dependency-graph rules — cycle detection, reachability, and the locked state
//! of a task that waits on others.
//!
//! Pure rules, per AGENTS.md: no Tauri, HTTP or database. The contract already
//! guards a single task against depending on itself (`Todo::validate`'s
//! `SelfDependency`), but a cycle across two or more tasks (A → B → A) can only
//! be seen by looking at the whole graph, which is what this module does. It is
//! the tested reference the view-side store mirrors: the store re-implements the
//! same checks in TypeScript so candidate culling and lock badges stay reactive,
//! and these functions keep that logic honest under `cargo test`.
//!
//! The graph is a task's `depends_on` edges read as data: each id maps to the
//! ids of the tasks it depends on (its prerequisites). An id that turns up as a
//! prerequisite but has no entry of its own is unresolvable — a task deleted or
//! not yet synced to this device — and is read as a leaf with no further edges.

use std::collections::{HashMap, HashSet};

/// The dependency edges of a task graph: each task id maps to the ids of the
/// tasks it depends on (its prerequisites).
pub type DependencyGraph = HashMap<String, Vec<String>>;

/// The prerequisites of one task, or an empty slice when the id is unresolvable.
fn prerequisites<'a>(graph: &'a DependencyGraph, id: &str) -> &'a [String] {
    graph.get(id).map(Vec::as_slice).unwrap_or(&[])
}

/// Every task reachable from `start` by following prerequisite edges — the
/// transitive prerequisites of `start`.
///
/// `start` itself is included only when a cycle leads back to it, which is what
/// lets [`has_cycle`] and [`introduces_cycle`] be phrased in terms of this walk.
pub fn transitive_prerequisites(graph: &DependencyGraph, start: &str) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut stack: Vec<&str> = prerequisites(graph, start)
        .iter()
        .map(String::as_str)
        .collect();
    while let Some(id) = stack.pop() {
        if !seen.insert(id.to_owned()) {
            continue;
        }
        for next in prerequisites(graph, id) {
            stack.push(next.as_str());
        }
    }
    seen
}

/// Every task that reaches `target` by following prerequisite edges — the
/// transitive dependents of `target` (the tasks that depend on it, directly or
/// through a chain).
///
/// This is the cull set for choosing a new prerequisite of `target`: adding the
/// edge `target → candidate` closes a loop exactly when `candidate` is one of
/// these, so the candidate list drops them before the user can pick one (and
/// `target` itself, a self-dependency).
pub fn transitive_dependents(graph: &DependencyGraph, target: &str) -> HashSet<String> {
    // Reverse adjacency, built once: for each prerequisite, the tasks that list
    // it. Walking that backwards from `target` is one pass over the graph rather
    // than a reachability check per node.
    let mut listed_by: HashMap<&str, Vec<&str>> = HashMap::new();
    for (id, deps) in graph {
        for dep in deps {
            listed_by.entry(dep.as_str()).or_default().push(id.as_str());
        }
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut stack: Vec<&str> = listed_by.get(target).cloned().unwrap_or_default();
    while let Some(id) = stack.pop() {
        if !seen.insert(id.to_owned()) {
            continue;
        }
        if let Some(parents) = listed_by.get(id) {
            stack.extend(parents.iter().copied());
        }
    }
    seen
}

/// Would making `dependent` depend on `prerequisite` introduce a cycle?
///
/// True when the two are the same task (a self-dependency the contract also
/// refuses) or `prerequisite` already reaches `dependent` through the graph, so
/// the new edge `dependent → prerequisite` would close a loop.
pub fn introduces_cycle(graph: &DependencyGraph, dependent: &str, prerequisite: &str) -> bool {
    if dependent == prerequisite {
        return true;
    }
    transitive_prerequisites(graph, prerequisite).contains(dependent)
}

/// Does the graph already contain a cycle among any of its tasks?
///
/// A graph has a cycle exactly when some task is among its own transitive
/// prerequisites. Phrased over [`transitive_prerequisites`] so there is one walk
/// to trust rather than a second, subtly different traversal.
pub fn has_cycle(graph: &DependencyGraph) -> bool {
    graph
        .keys()
        .any(|id| transitive_prerequisites(graph, id).contains(id.as_str()))
}

/// A task's progress against its prerequisites: how many are satisfied, how many
/// there are, and whether it is still locked.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DependencyLock {
    /// Prerequisites already satisfied — completed, or unresolvable.
    pub done: usize,
    /// How many prerequisites the task lists.
    pub total: usize,
    /// Whether any prerequisite is still outstanding.
    pub locked: bool,
}

/// Reads a task's lock state from its prerequisite ids and the graph around it.
///
/// A prerequisite counts as satisfied when it is completed, or when it cannot be
/// resolved on this device at all — a task deleted or not yet synced here. An
/// unresolvable prerequisite is read as satisfied rather than blocking (编排者
/// 裁决 2026-07-24): a task waiting on something this device will never see
/// complete would otherwise be locked forever with no way out, whereas counting
/// it satisfied only ever unlocks early — recoverable, and the user can re-set
/// the dependency.
///
/// `known` is the set of task ids that exist on this device; `completed` the
/// subset of those that are done. A prerequisite is satisfied iff it is absent
/// from `known` (unresolvable) or present in `completed`.
pub fn dependency_lock(
    depends_on: &[String],
    known: &HashSet<String>,
    completed: &HashSet<String>,
) -> DependencyLock {
    let total = depends_on.len();
    let done = depends_on
        .iter()
        .filter(|id| !known.contains(id.as_str()) || completed.contains(id.as_str()))
        .count();
    DependencyLock {
        done,
        total,
        locked: done < total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a graph from `(task, [prerequisites])` pairs.
    fn graph(edges: &[(&str, &[&str])]) -> DependencyGraph {
        edges
            .iter()
            .map(|(id, deps)| {
                (
                    (*id).to_owned(),
                    deps.iter().map(|dep| (*dep).to_owned()).collect(),
                )
            })
            .collect()
    }

    fn ids(set: &HashSet<String>) -> Vec<String> {
        let mut names: Vec<String> = set.iter().cloned().collect();
        names.sort();
        names
    }

    fn set(members: &[&str]) -> HashSet<String> {
        members.iter().map(|member| (*member).to_owned()).collect()
    }

    #[test]
    fn transitive_prerequisites_follow_the_chain() {
        // a → b → c: a's prerequisites are b and c, transitively.
        let graph = graph(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
        assert_eq!(ids(&transitive_prerequisites(&graph, "a")), vec!["b", "c"]);
        assert_eq!(ids(&transitive_prerequisites(&graph, "b")), vec!["c"]);
        assert!(transitive_prerequisites(&graph, "c").is_empty());
    }

    #[test]
    fn an_unresolvable_prerequisite_is_a_leaf() {
        // b is listed but has no entry of its own (deleted / not synced). It is a
        // leaf: reachable, but leads nowhere and closes no loop.
        let graph = graph(&[("a", &["b"])]);
        assert_eq!(ids(&transitive_prerequisites(&graph, "a")), vec!["b"]);
        assert!(!has_cycle(&graph));
    }

    #[test]
    fn a_direct_back_edge_is_a_cycle() {
        // a depends on b; adding b → a would close the two-task loop.
        let graph = graph(&[("a", &["b"]), ("b", &[])]);
        assert!(introduces_cycle(&graph, "b", "a"));
        // The other direction is the edge that already exists — no new loop.
        assert!(!introduces_cycle(&graph, "a", "b"));
    }

    #[test]
    fn a_transitive_back_edge_is_a_cycle() {
        // a → b → c. Making a a prerequisite of c (c → a) closes a → b → c → a.
        let graph = graph(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
        assert!(introduces_cycle(&graph, "c", "a"));
        // Adding a → c directly is redundant with the chain, not a cycle: c
        // reaches nothing, so no loop can close back to a.
        assert!(!introduces_cycle(&graph, "a", "c"));
    }

    #[test]
    fn depending_on_self_is_always_a_cycle() {
        let graph = graph(&[("a", &[])]);
        assert!(introduces_cycle(&graph, "a", "a"));
    }

    #[test]
    fn an_unrelated_prerequisite_closes_no_loop() {
        // Two separate chains: joining across them never cycles.
        let graph = graph(&[("a", &["b"]), ("b", &[]), ("x", &["y"]), ("y", &[])]);
        assert!(!introduces_cycle(&graph, "a", "x"));
        assert!(!introduces_cycle(&graph, "b", "y"));
    }

    #[test]
    fn transitive_dependents_are_the_cull_set() {
        // a → b → c. c's dependents are a and b (both wait on c through the
        // chain), so both would cycle if made to depend on c.
        let graph = graph(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
        assert_eq!(ids(&transitive_dependents(&graph, "c")), vec!["a", "b"]);
        assert_eq!(ids(&transitive_dependents(&graph, "b")), vec!["a"]);
        assert!(transitive_dependents(&graph, "a").is_empty());
        // The cull set is exactly the candidates that introduce a cycle.
        for candidate in ["a", "b"] {
            assert!(introduces_cycle(&graph, "c", candidate));
        }
    }

    #[test]
    fn has_cycle_reads_the_whole_graph() {
        let acyclic = graph(&[("a", &["b"]), ("b", &["c"]), ("c", &[])]);
        assert!(!has_cycle(&acyclic));

        // a → b → a is a cycle no single task's self-check would catch.
        let cyclic = graph(&[("a", &["b"]), ("b", &["a"])]);
        assert!(has_cycle(&cyclic));

        // A longer loop, and a self-loop.
        let long = graph(&[("a", &["b"]), ("b", &["c"]), ("c", &["a"])]);
        assert!(has_cycle(&long));
        let selfish = graph(&[("a", &["a"])]);
        assert!(has_cycle(&selfish));
    }

    #[test]
    fn lock_is_open_when_every_prerequisite_is_done() {
        let known = set(&["a", "b", "c"]);
        let completed = set(&["b", "c"]);
        let lock = dependency_lock(
            &["b".to_owned(), "c".to_owned()],
            &known,
            &completed,
        );
        assert_eq!(
            lock,
            DependencyLock {
                done: 2,
                total: 2,
                locked: false,
            }
        );
    }

    #[test]
    fn lock_holds_while_a_prerequisite_is_pending() {
        let known = set(&["a", "b", "c"]);
        let completed = set(&["b"]);
        let lock = dependency_lock(
            &["b".to_owned(), "c".to_owned()],
            &known,
            &completed,
        );
        assert_eq!(
            lock,
            DependencyLock {
                done: 1,
                total: 2,
                locked: true,
            }
        );
    }

    #[test]
    fn an_unresolvable_prerequisite_counts_as_satisfied() {
        // "ghost" is not in `known`: a task deleted or not yet synced here. Per
        // the 2026-07-24 ruling it is satisfied, not blocking, so a lone
        // unresolvable prerequisite unlocks the task rather than trapping it.
        let known = set(&["a"]);
        let completed = HashSet::new();
        let lock = dependency_lock(&["ghost".to_owned()], &known, &completed);
        assert_eq!(
            lock,
            DependencyLock {
                done: 1,
                total: 1,
                locked: false,
            }
        );

        // Mixed: one real pending prerequisite still locks even beside a
        // satisfied ghost.
        let known = set(&["a", "real"]);
        let lock = dependency_lock(
            &["ghost".to_owned(), "real".to_owned()],
            &known,
            &completed,
        );
        assert_eq!(
            lock,
            DependencyLock {
                done: 1,
                total: 2,
                locked: true,
            }
        );
    }

    #[test]
    fn a_task_with_no_prerequisites_is_never_locked() {
        let lock = dependency_lock(&[], &HashSet::new(), &HashSet::new());
        assert_eq!(lock, DependencyLock::default());
        assert!(!lock.locked);
    }
}
