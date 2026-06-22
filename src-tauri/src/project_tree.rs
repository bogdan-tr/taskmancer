use crate::project::Project;

/// Returns the direct children of `parent_id` (or every top-level project,
/// if `parent_id` is `None`), in the order they appear in `projects`.
#[allow(dead_code)]
pub fn children_of<'a>(projects: &'a [Project], parent_id: Option<&str>) -> Vec<&'a Project> {
    projects
        .iter()
        .filter(|p| p.parent_id.as_deref() == parent_id)
        .collect()
}

/// Returns the ancestors of the project identified by `id`, nearest-first,
/// ending at the root. Empty if `id` doesn't exist in `projects` or names a
/// top-level project.
#[allow(dead_code)]
pub fn ancestors_of<'a>(projects: &'a [Project], id: &str) -> Vec<&'a Project> {
    let mut result = Vec::new();
    let mut current_id = projects
        .iter()
        .find(|p| p.id == id)
        .and_then(|p| p.parent_id.clone());

    while let Some(ancestor_id) = current_id {
        let Some(ancestor) = projects.iter().find(|p| p.id == ancestor_id) else {
            break;
        };
        result.push(ancestor);
        current_id = ancestor.parent_id.clone();
    }

    result
}

/// Returns the project identified by `id` (if found) followed by its
/// ancestors, nearest-first — the full settings-resolution chain for that
/// project, ending at the root. Empty if `id` doesn't exist in `projects`.
#[allow(dead_code)]
pub fn self_and_ancestors<'a>(projects: &'a [Project], id: &str) -> Vec<&'a Project> {
    let mut result: Vec<&Project> = projects.iter().find(|p| p.id == id).into_iter().collect();
    result.extend(ancestors_of(projects, id));
    result
}

/// Returns every transitive descendant of the project identified by `id`
/// (children, grandchildren, ...). Order is not guaranteed to be any
/// particular traversal order — callers needing a specific order should sort
/// the result themselves.
#[allow(dead_code)]
pub fn descendants_of<'a>(projects: &'a [Project], id: &str) -> Vec<&'a Project> {
    let mut result: Vec<&Project> = Vec::new();
    let mut frontier: Vec<String> = vec![id.to_string()];

    while let Some(current_id) = frontier.pop() {
        for child in children_of(projects, Some(current_id.as_str())) {
            result.push(child);
            frontier.push(child.id.clone());
        }
    }

    result
}

/// Returns `true` if making `new_parent_id` the parent of `moving_id` would
/// create a cycle — i.e. `new_parent_id` is `moving_id` itself, or is one of
/// `moving_id`'s current descendants.
#[allow(dead_code)]
pub fn would_create_cycle(projects: &[Project], moving_id: &str, new_parent_id: &str) -> bool {
    if moving_id == new_parent_id {
        return true;
    }
    descendants_of(projects, moving_id)
        .iter()
        .any(|p| p.id == new_parent_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a small fixture tree:
    /// ```text
    /// root_a (top-level)
    /// ├── child_a1
    /// │   └── grandchild_a1a
    /// └── child_a2
    /// root_b (top-level, no children)
    /// ```
    fn fixture_tree() -> Vec<Project> {
        let mut root_a = Project::new("Root A".to_string(), "#111111".to_string(), 1);
        root_a.id = "root_a".to_string();

        let mut root_b = Project::new("Root B".to_string(), "#222222".to_string(), 2);
        root_b.id = "root_b".to_string();

        let mut child_a1 = Project::new("Child A1".to_string(), "#333333".to_string(), 1);
        child_a1.id = "child_a1".to_string();
        child_a1.parent_id = Some("root_a".to_string());

        let mut child_a2 = Project::new("Child A2".to_string(), "#444444".to_string(), 2);
        child_a2.id = "child_a2".to_string();
        child_a2.parent_id = Some("root_a".to_string());

        let mut grandchild_a1a =
            Project::new("Grandchild A1a".to_string(), "#555555".to_string(), 1);
        grandchild_a1a.id = "grandchild_a1a".to_string();
        grandchild_a1a.parent_id = Some("child_a1".to_string());

        vec![root_a, root_b, child_a1, child_a2, grandchild_a1a]
    }

    #[test]
    fn children_of_none_returns_top_level_projects() {
        let projects = fixture_tree();

        let children = children_of(&projects, None);

        let ids: Vec<&str> = children.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["root_a", "root_b"]);
    }

    #[test]
    fn children_of_returns_direct_children_only() {
        let projects = fixture_tree();

        let children = children_of(&projects, Some("root_a"));

        let ids: Vec<&str> = children.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["child_a1", "child_a2"]);
    }

    #[test]
    fn children_of_returns_empty_for_a_leaf() {
        let projects = fixture_tree();

        let children = children_of(&projects, Some("child_a2"));

        assert!(children.is_empty());
    }

    #[test]
    fn ancestors_of_top_level_project_is_empty() {
        let projects = fixture_tree();

        let ancestors = ancestors_of(&projects, "root_a");

        assert!(ancestors.is_empty());
    }

    #[test]
    fn ancestors_of_returns_nearest_first() {
        let projects = fixture_tree();

        let ancestors = ancestors_of(&projects, "grandchild_a1a");

        let ids: Vec<&str> = ancestors.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["child_a1", "root_a"]);
    }

    #[test]
    fn ancestors_of_missing_id_is_empty() {
        let projects = fixture_tree();

        let ancestors = ancestors_of(&projects, "does_not_exist");

        assert!(ancestors.is_empty());
    }

    #[test]
    fn ancestors_of_stops_at_a_dangling_parent_id() {
        let mut projects = fixture_tree();
        // Simulate a parent that was deleted without cleaning up this
        // reference: child_a1's parent_id points at a project that no
        // longer exists.
        projects[2].parent_id = Some("deleted_project".to_string());

        let ancestors = ancestors_of(&projects, "grandchild_a1a");

        let ids: Vec<&str> = ancestors.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["child_a1"]);
    }

    #[test]
    fn self_and_ancestors_includes_self_first() {
        let projects = fixture_tree();

        let chain = self_and_ancestors(&projects, "grandchild_a1a");

        let ids: Vec<&str> = chain.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["grandchild_a1a", "child_a1", "root_a"]);
    }

    #[test]
    fn self_and_ancestors_for_missing_id_is_empty() {
        let projects = fixture_tree();

        let chain = self_and_ancestors(&projects, "does_not_exist");

        assert!(chain.is_empty());
    }

    #[test]
    fn descendants_of_returns_all_levels() {
        let projects = fixture_tree();

        let descendants = descendants_of(&projects, "root_a");

        let mut ids: Vec<&str> = descendants.iter().map(|p| p.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["child_a1", "child_a2", "grandchild_a1a"]);
    }

    #[test]
    fn descendants_of_a_leaf_is_empty() {
        let projects = fixture_tree();

        let descendants = descendants_of(&projects, "grandchild_a1a");

        assert!(descendants.is_empty());
    }

    #[test]
    fn descendants_of_an_unrelated_root_is_empty() {
        let projects = fixture_tree();

        let descendants = descendants_of(&projects, "root_b");

        assert!(descendants.is_empty());
    }

    #[test]
    fn would_create_cycle_when_moving_under_self() {
        let projects = fixture_tree();

        assert!(would_create_cycle(&projects, "root_a", "root_a"));
    }

    #[test]
    fn would_create_cycle_when_moving_under_own_descendant() {
        let projects = fixture_tree();

        assert!(would_create_cycle(&projects, "root_a", "grandchild_a1a"));
    }

    #[test]
    fn would_not_create_cycle_when_moving_to_an_unrelated_project() {
        let projects = fixture_tree();

        assert!(!would_create_cycle(&projects, "child_a1", "root_b"));
    }

    #[test]
    fn would_not_create_cycle_when_moving_to_top_level() {
        let projects = fixture_tree();

        // Promoting child_a1 to top-level is modeled as re-parenting under
        // a synthetic id that isn't in the tree at all — would_create_cycle
        // only guards against id-based cycles, so this is just confirming
        // an arbitrary unrelated id is never flagged.
        assert!(!would_create_cycle(&projects, "child_a1", "unrelated_id"));
    }
}
