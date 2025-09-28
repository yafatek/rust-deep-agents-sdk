use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Snapshot of agent state shared between runtime, planners, and tools.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AgentStateSnapshot {
    pub todos: Vec<TodoItem>,
    pub files: BTreeMap<String, String>,
    pub scratchpad: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl TodoItem {
    pub fn pending(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            status: TodoStatus::Pending,
        }
    }
}

impl AgentStateSnapshot {
    /// Merge another state snapshot into this one using reducer logic.
    pub fn merge(&mut self, other: AgentStateSnapshot) {
        // Files reducer: merge dictionaries (equivalent to {**l, **r})
        self.files.extend(other.files);

        // Todos reducer: replace with other if not empty, otherwise keep current
        if !other.todos.is_empty() {
            self.todos = other.todos;
        }

        // Scratchpad reducer: merge dictionaries
        self.scratchpad.extend(other.scratchpad);
    }

    /// File reducer function matching Python's file_reducer behavior.
    /// Merges two optional file dictionaries, handling None values appropriately.
    pub fn reduce_files(
        left: Option<BTreeMap<String, String>>,
        right: Option<BTreeMap<String, String>>,
    ) -> Option<BTreeMap<String, String>> {
        match (left, right) {
            (None, None) => None,
            (Some(l), None) => Some(l),
            (None, Some(r)) => Some(r),
            (Some(mut l), Some(r)) => {
                l.extend(r); // Equivalent to Python's {**l, **r}
                Some(l)
            }
        }
    }

    /// Create a new state with merged files, handling None values.
    pub fn with_merged_files(&self, new_files: Option<BTreeMap<String, String>>) -> Self {
        let mut result = self.clone();
        if let Some(files) = new_files {
            result.files.extend(files);
        }
        result
    }

    pub fn with_updated_todos(&self, new_todos: Vec<TodoItem>) -> Self {
        if new_todos.is_empty() {
            self.clone()
        } else {
            let mut result = self.clone();
            result.todos = new_todos;
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_reducer_both_none() {
        let result = AgentStateSnapshot::reduce_files(None, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_file_reducer_left_some_right_none() {
        let mut left = BTreeMap::new();
        left.insert("file1.txt".to_string(), "content1".to_string());

        let result = AgentStateSnapshot::reduce_files(Some(left.clone()), None);
        assert_eq!(result, Some(left));
    }

    #[test]
    fn test_file_reducer_left_none_right_some() {
        let mut right = BTreeMap::new();
        right.insert("file2.txt".to_string(), "content2".to_string());

        let result = AgentStateSnapshot::reduce_files(None, Some(right.clone()));
        assert_eq!(result, Some(right));
    }

    #[test]
    fn test_file_reducer_both_some_merges() {
        let mut left = BTreeMap::new();
        left.insert("file1.txt".to_string(), "content1".to_string());
        left.insert("shared.txt".to_string(), "old_content".to_string());

        let mut right = BTreeMap::new();
        right.insert("file2.txt".to_string(), "content2".to_string());
        right.insert("shared.txt".to_string(), "new_content".to_string());

        let result = AgentStateSnapshot::reduce_files(Some(left), Some(right)).unwrap();

        // Should have all files, with right overwriting left for conflicts
        assert_eq!(result.get("file1.txt").unwrap(), "content1");
        assert_eq!(result.get("file2.txt").unwrap(), "content2");
        assert_eq!(result.get("shared.txt").unwrap(), "new_content"); // Right wins
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_merge_combines_states() {
        let mut state1 = AgentStateSnapshot::default();
        state1
            .files
            .insert("file1.txt".to_string(), "content1".to_string());
        state1.todos.push(TodoItem::pending("task1"));
        state1
            .scratchpad
            .insert("key1".to_string(), serde_json::json!("value1"));

        let mut state2 = AgentStateSnapshot::default();
        state2
            .files
            .insert("file2.txt".to_string(), "content2".to_string());
        state2.todos.push(TodoItem::pending("task2"));
        state2
            .scratchpad
            .insert("key2".to_string(), serde_json::json!("value2"));

        let mut merged = state1.clone();
        merged.merge(state2);

        // Files should be merged
        assert_eq!(merged.files.len(), 2);
        assert_eq!(merged.files.get("file1.txt").unwrap(), "content1");
        assert_eq!(merged.files.get("file2.txt").unwrap(), "content2");

        // Todos should be replaced (not empty)
        assert_eq!(merged.todos.len(), 1);
        assert_eq!(merged.todos[0].content, "task2");

        // Scratchpad should be merged
        assert_eq!(merged.scratchpad.len(), 2);
        assert_eq!(merged.scratchpad.get("key1").unwrap(), "value1");
        assert_eq!(merged.scratchpad.get("key2").unwrap(), "value2");
    }

    #[test]
    fn test_merge_empty_todos_preserves_existing() {
        let mut state1 = AgentStateSnapshot::default();
        state1.todos.push(TodoItem::pending("task1"));

        let state2 = AgentStateSnapshot::default(); // Empty todos

        let mut merged = state1.clone();
        merged.merge(state2);

        // Should preserve original todos since new ones are empty
        assert_eq!(merged.todos.len(), 1);
        assert_eq!(merged.todos[0].content, "task1");
    }

    #[test]
    fn test_with_merged_files() {
        let mut state = AgentStateSnapshot::default();
        state
            .files
            .insert("existing.txt".to_string(), "existing".to_string());

        let mut new_files = BTreeMap::new();
        new_files.insert("new.txt".to_string(), "new_content".to_string());
        new_files.insert("existing.txt".to_string(), "updated".to_string()); // Should overwrite

        let result = state.with_merged_files(Some(new_files));

        assert_eq!(result.files.len(), 2);
        assert_eq!(result.files.get("existing.txt").unwrap(), "updated");
        assert_eq!(result.files.get("new.txt").unwrap(), "new_content");
    }

    #[test]
    fn test_with_updated_todos() {
        let mut state = AgentStateSnapshot::default();
        state.todos.push(TodoItem::pending("old_task"));

        let new_todos = vec![
            TodoItem::pending("new_task1"),
            TodoItem::pending("new_task2"),
        ];

        let result = state.with_updated_todos(new_todos);

        assert_eq!(result.todos.len(), 2);
        assert_eq!(result.todos[0].content, "new_task1");
        assert_eq!(result.todos[1].content, "new_task2");
    }

    #[test]
    fn test_with_updated_todos_empty_preserves_existing() {
        let mut state = AgentStateSnapshot::default();
        state.todos.push(TodoItem::pending("existing_task"));

        let result = state.with_updated_todos(vec![]);

        // Should preserve existing todos when new list is empty
        assert_eq!(result.todos.len(), 1);
        assert_eq!(result.todos[0].content, "existing_task");
    }
}
