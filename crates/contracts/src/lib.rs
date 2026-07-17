use serde::{Deserialize, Serialize};
use specta::Type;
use ts_rs::TS;

pub type SyncCursor = u32;

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub status: TodoStatus,
    pub created_at: String,
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub due_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reminder_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum TodoStatus {
    Open,
    Completed,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct TodoPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub status: Option<TodoStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub due_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub completed_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum SyncOperationKind {
    Upsert,
    Delete,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct TodoSyncOperation {
    pub operation_id: String,
    pub todo_id: String,
    pub kind: SyncOperationKind,
    pub occurred_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub patch: Option<TodoPatch>,
}

impl TodoSyncOperation {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if let Some(title) = self.patch.as_ref().and_then(|patch| patch.title.as_ref()) {
            let trimmed = title.trim();
            if trimmed.is_empty() || trimmed.chars().count() > 500 {
                return Err(ContractValidationError::InvalidTitle);
            }
        }

        match (&self.kind, &self.patch) {
            (SyncOperationKind::Upsert, None) => Err(ContractValidationError::MissingUpsertPatch),
            (SyncOperationKind::Delete, Some(_)) => Err(ContractValidationError::DeleteHasPatch),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct TodoSyncChange {
    pub operation_id: String,
    pub todo_id: String,
    pub kind: SyncOperationKind,
    pub occurred_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub patch: Option<TodoPatch>,
    pub revision: SyncCursor,
}

impl TodoSyncChange {
    pub fn from_operation(operation: TodoSyncOperation, revision: SyncCursor) -> Self {
        Self {
            operation_id: operation.operation_id,
            todo_id: operation.todo_id,
            kind: operation.kind,
            occurred_at: operation.occurred_at,
            patch: operation.patch,
            revision,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct SyncRequest {
    pub device_id: String,
    pub cursor: SyncCursor,
    pub operations: Vec<TodoSyncOperation>,
}

impl SyncRequest {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.operations.len() > 100 {
            return Err(ContractValidationError::TooManyOperations);
        }

        self.operations
            .iter()
            .try_for_each(TodoSyncOperation::validate)
    }
}

#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SyncResponse {
    pub acknowledged_operation_ids: Vec<String>,
    pub changes: Vec<TodoSyncChange>,
    pub next_cursor: SyncCursor,
    pub server_time: String,
}

#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub time: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractValidationError {
    DeleteHasPatch,
    InvalidTitle,
    MissingUpsertPatch,
    TooManyOperations,
}

impl std::fmt::Display for ContractValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::DeleteHasPatch => "delete operations must not include a patch",
            Self::InvalidTitle => "a title must contain 1 to 500 non-whitespace characters",
            Self::MissingUpsertPatch => "upsert operations require a patch",
            Self::TooManyOperations => "sync requests support at most 100 operations",
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for ContractValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_operation_rejects_a_patch() {
        let operation = TodoSyncOperation {
            operation_id: "operation-1".to_owned(),
            todo_id: "todo-1".to_owned(),
            kind: SyncOperationKind::Delete,
            occurred_at: "2026-07-16T00:00:00Z".to_owned(),
            patch: Some(TodoPatch::default()),
        };

        assert_eq!(
            operation.validate(),
            Err(ContractValidationError::DeleteHasPatch)
        );
    }
}
