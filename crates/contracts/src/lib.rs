use serde::{Deserialize, Serialize};
use specta::Type;
use ts_rs::TS;

pub type SyncCursor = u32;

/// Longest accepted title, in characters, for a todo or one of its subtasks.
///
/// Public because the title is the one field a user types today: the view layer
/// exports this number through the generated bindings and stops the input at
/// the same limit, so the app never accepts a title storage would refuse.
pub const MAX_TITLE_CHARS: usize = 500;
/// Longest accepted free-text note on a todo.
const MAX_NOTES_CHARS: usize = 20_000;
/// Longest accepted attachment location.
const MAX_URL_CHARS: usize = 2_048;
/// Longest accepted list-valued field (tags, subtasks, attachments, dependencies).
const MAX_LIST_ENTRIES: usize = 200;
/// Largest accepted repeat interval, which covers "every N days" up to a year.
const MAX_RECURRENCE_INTERVAL: u32 = 999;

/// A task.
///
/// Every field beyond the original six is optional or list-valued with a serde
/// default, which is what keeps a payload written by an earlier build readable:
/// the fields it never knew about simply come back empty. `deny_unknown_fields`
/// only bites in the other direction — a payload from a *newer* build — and the
/// repository has no version negotiation by design (AGENTS.md), so bindings are
/// regenerated and callers are updated together with the contract.
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
    #[ts(optional = nullable)]
    pub due_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub reminder_at: Option<String>,
    /// Free-text description, searchable alongside the title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub notes: Option<String>,
    /// Local date the task becomes actionable; before it the task stays out of
    /// the way instead of demanding attention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub start_date: Option<String>,
    /// Start of a timed block, as an instant (the due date stays a plain date).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub starts_at: Option<String>,
    /// End of a timed block; only meaningful together with `starts_at`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub ends_at: Option<String>,
    /// Planned effort in minutes, compared against recorded focus time later.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub estimated_minutes: Option<u32>,
    /// Repeat rule; absent means the task happens once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub recurrence: Option<RecurrenceRule>,
    /// The list this task belongs to; absent means the inbox.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub list_id: Option<String>,
    /// Eisenhower axes. Kept separate and tri-state on purpose: `None` is "the
    /// user has not said", which is what lets the urgency default be inferred
    /// from the due date without overwriting a deliberate choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub important: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub urgent: Option<bool>,
    /// Manual position within a list. Fractional so a drag between two rows
    /// only writes the row that moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub sort_order: Option<f64>,
    /// Tags applied to this task, by tag id.
    #[serde(default)]
    pub tag_ids: Vec<String>,
    /// Checklist inside the task; the order of the vector is the display order.
    #[serde(default)]
    pub subtasks: Vec<Subtask>,
    /// Files and links hanging off the task.
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    /// Tasks that must be done before this one.
    #[serde(default)]
    pub depends_on: Vec<String>,
}

impl Todo {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        validate_id(&self.id)?;
        validate_title(&self.title)?;

        if let Some(notes) = &self.notes {
            if notes.chars().count() > MAX_NOTES_CHARS {
                return Err(ContractValidationError::InvalidNotes);
            }
        }

        if self.ends_at.is_some() && self.starts_at.is_none() {
            return Err(ContractValidationError::InvalidTimeRange);
        }

        if self.estimated_minutes == Some(0) {
            return Err(ContractValidationError::InvalidEstimate);
        }

        if let Some(recurrence) = &self.recurrence {
            recurrence.validate()?;
        }

        if let Some(list_id) = &self.list_id {
            validate_id(list_id)?;
        }

        validate_id_list(&self.tag_ids)?;
        validate_subtasks(&self.subtasks)?;
        validate_attachments(&self.attachments)?;
        validate_id_list(&self.depends_on)?;
        if self.depends_on.contains(&self.id) {
            return Err(ContractValidationError::SelfDependency);
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum TodoStatus {
    Open,
    Completed,
}

/// One step of a task's checklist.
#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Subtask {
    pub id: String,
    pub title: String,
    pub done: bool,
}

/// A file or link attached to a task.
#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub kind: AttachmentKind,
    /// Where the attachment lives: the URL for a link, the file location for a
    /// file.
    pub url: String,
    /// Label shown instead of the raw location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum AttachmentKind {
    Link,
    File,
}

/// How a task repeats.
///
/// The rule is flat rather than a nested variant per frequency because it is
/// stored, synced and edited as one unit; the combinations that do not make
/// sense are rejected by `validate` instead of being made unrepresentable, which
/// keeps the generated TypeScript a single object type.
#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct RecurrenceRule {
    pub frequency: RecurrenceFrequency,
    /// Repeat every `interval` units of `frequency`.
    pub interval: u32,
    /// For weekly rules, the days it lands on (e.g. Monday/Wednesday/Friday).
    /// Empty means "the same weekday as the task's due date".
    #[serde(default)]
    pub weekdays: Vec<Weekday>,
    /// For monthly rules, the day of the month (1–31).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub month_day: Option<u32>,
    /// For monthly rules, "the last day of the month" — the one position that
    /// cannot be written as a fixed day number.
    pub on_last_day: bool,
    /// Which calendar the rule is counted in.
    pub calendar: RecurrenceCalendar,
    /// Local date the repetition stops on, inclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub until: Option<String>,
    /// Number of occurrences the repetition stops after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub count: Option<u32>,
}

impl RecurrenceRule {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if self.interval == 0 || self.interval > MAX_RECURRENCE_INTERVAL {
            return Err(ContractValidationError::InvalidRecurrence);
        }

        for (index, weekday) in self.weekdays.iter().enumerate() {
            if self.weekdays[..index]
                .iter()
                .any(|earlier| earlier == weekday)
            {
                return Err(ContractValidationError::DuplicateEntry);
            }
        }

        if let Some(day) = self.month_day {
            if day == 0 || day > 31 || self.on_last_day {
                return Err(ContractValidationError::InvalidRecurrence);
            }
        }

        if self.count == Some(0) {
            return Err(ContractValidationError::InvalidRecurrence);
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    /// Every working day, which is holiday-aware rather than "Monday to Friday".
    Workday,
}

#[derive(Clone, Debug, Deserialize, Serialize, TS, Type)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum RecurrenceCalendar {
    Gregorian,
    Lunar,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS, Type)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// The fields one sync operation changes.
///
/// An absent field means "unchanged"; a present one replaces the stored value,
/// list-valued fields included (a tag set is sent whole, not as a diff).
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub reminder_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub start_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub starts_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub ends_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub estimated_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub recurrence: Option<RecurrenceRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub list_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub important: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub urgent: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub sort_order: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tag_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub subtasks: Option<Vec<Subtask>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub attachments: Option<Vec<Attachment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub depends_on: Option<Vec<String>>,
}

impl TodoPatch {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if let Some(title) = &self.title {
            validate_title(title)?;
        }

        if let Some(notes) = &self.notes {
            if notes.chars().count() > MAX_NOTES_CHARS {
                return Err(ContractValidationError::InvalidNotes);
            }
        }

        if self.estimated_minutes == Some(0) {
            return Err(ContractValidationError::InvalidEstimate);
        }

        if let Some(recurrence) = &self.recurrence {
            recurrence.validate()?;
        }

        if let Some(list_id) = &self.list_id {
            validate_id(list_id)?;
        }

        if let Some(tag_ids) = &self.tag_ids {
            validate_id_list(tag_ids)?;
        }
        if let Some(subtasks) = &self.subtasks {
            validate_subtasks(subtasks)?;
        }
        if let Some(attachments) = &self.attachments {
            validate_attachments(attachments)?;
        }
        if let Some(depends_on) = &self.depends_on {
            validate_id_list(depends_on)?;
        }

        Ok(())
    }
}

fn validate_id(value: &str) -> Result<(), ContractValidationError> {
    if value.trim().is_empty() {
        return Err(ContractValidationError::InvalidId);
    }
    Ok(())
}

fn validate_title(value: &str) -> Result<(), ContractValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_TITLE_CHARS {
        return Err(ContractValidationError::InvalidTitle);
    }
    Ok(())
}

/// Ids must be usable and distinct; the lists are short enough that comparing
/// against the entries already seen beats pulling in a set.
fn validate_id_list(ids: &[String]) -> Result<(), ContractValidationError> {
    if ids.len() > MAX_LIST_ENTRIES {
        return Err(ContractValidationError::TooManyEntries);
    }

    for (index, id) in ids.iter().enumerate() {
        validate_id(id)?;
        if ids[..index].contains(id) {
            return Err(ContractValidationError::DuplicateEntry);
        }
    }
    Ok(())
}

fn validate_subtasks(subtasks: &[Subtask]) -> Result<(), ContractValidationError> {
    if subtasks.len() > MAX_LIST_ENTRIES {
        return Err(ContractValidationError::TooManyEntries);
    }

    for (index, subtask) in subtasks.iter().enumerate() {
        validate_id(&subtask.id)?;
        validate_title(&subtask.title)?;
        if subtasks[..index]
            .iter()
            .any(|earlier| earlier.id == subtask.id)
        {
            return Err(ContractValidationError::DuplicateEntry);
        }
    }
    Ok(())
}

fn validate_attachments(attachments: &[Attachment]) -> Result<(), ContractValidationError> {
    if attachments.len() > MAX_LIST_ENTRIES {
        return Err(ContractValidationError::TooManyEntries);
    }

    for (index, attachment) in attachments.iter().enumerate() {
        validate_id(&attachment.id)?;
        let url = attachment.url.trim();
        if url.is_empty() || url.chars().count() > MAX_URL_CHARS {
            return Err(ContractValidationError::InvalidAttachment);
        }
        if attachments[..index]
            .iter()
            .any(|earlier| earlier.id == attachment.id)
        {
            return Err(ContractValidationError::DuplicateEntry);
        }
    }
    Ok(())
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
        if let Some(patch) = &self.patch {
            patch.validate()?;
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
    DuplicateEntry,
    InvalidAttachment,
    InvalidEstimate,
    InvalidId,
    InvalidNotes,
    InvalidRecurrence,
    InvalidTimeRange,
    InvalidTitle,
    MissingUpsertPatch,
    SelfDependency,
    TooManyEntries,
    TooManyOperations,
}

impl std::fmt::Display for ContractValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::DeleteHasPatch => "delete operations must not include a patch",
            Self::DuplicateEntry => "a list field must not repeat an entry",
            Self::InvalidAttachment => "an attachment needs a location of 1 to 2048 characters",
            Self::InvalidEstimate => "an estimate must be at least one minute",
            Self::InvalidId => "an id must contain non-whitespace characters",
            Self::InvalidNotes => "notes support at most 20000 characters",
            Self::InvalidRecurrence => "the recurrence rule is not a usable combination",
            Self::InvalidTimeRange => "an end time requires a start time",
            Self::InvalidTitle => "a title must contain 1 to 500 non-whitespace characters",
            Self::MissingUpsertPatch => "upsert operations require a patch",
            Self::SelfDependency => "a task cannot depend on itself",
            Self::TooManyEntries => "a list field supports at most 200 entries",
            Self::TooManyOperations => "sync requests support at most 100 operations",
        };

        formatter.write_str(message)
    }
}

impl std::error::Error for ContractValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn todo() -> Todo {
        Todo {
            id: "todo-1".to_owned(),
            title: "write the contract".to_owned(),
            status: TodoStatus::Open,
            created_at: "2026-07-16T00:00:00Z".to_owned(),
            completed_at: None,
            due_date: None,
            reminder_at: None,
            notes: None,
            start_date: None,
            starts_at: None,
            ends_at: None,
            estimated_minutes: None,
            recurrence: None,
            list_id: None,
            important: None,
            urgent: None,
            sort_order: None,
            tag_ids: Vec::new(),
            subtasks: Vec::new(),
            attachments: Vec::new(),
            depends_on: Vec::new(),
        }
    }

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

    #[test]
    fn a_payload_without_the_new_fields_still_decodes() {
        // Exactly the shape an earlier build wrote, which is what every stored
        // row and every parked write in the wild looks like.
        let json = r#"{
            "id": "todo-1",
            "title": "written by an older build",
            "status": "open",
            "createdAt": "2026-07-16T00:00:00Z",
            "completedAt": null,
            "dueDate": "2026-07-20"
        }"#;

        let decoded: Todo = serde_json::from_str(json).expect("decode a v1 payload");

        assert_eq!(decoded.due_date.as_deref(), Some("2026-07-20"));
        assert!(decoded.tag_ids.is_empty());
        assert!(decoded.subtasks.is_empty());
        assert!(decoded.attachments.is_empty());
        assert!(decoded.depends_on.is_empty());
        assert_eq!(decoded.important, None);
        assert!(decoded.recurrence.is_none());
        decoded.validate().expect("a v1 payload stays valid");
    }

    #[test]
    fn the_new_fields_round_trip_through_json() {
        let mut original = todo();
        original.notes = Some("with a note".to_owned());
        original.start_date = Some("2026-07-18".to_owned());
        original.starts_at = Some("2026-07-20T09:00:00Z".to_owned());
        original.ends_at = Some("2026-07-20T10:00:00Z".to_owned());
        original.estimated_minutes = Some(60);
        original.list_id = Some("list-1".to_owned());
        original.important = Some(true);
        original.urgent = Some(false);
        original.sort_order = Some(1.5);
        original.tag_ids = vec!["tag-1".to_owned()];
        original.depends_on = vec!["todo-2".to_owned()];
        original.subtasks = vec![Subtask {
            id: "step-1".to_owned(),
            title: "first step".to_owned(),
            done: true,
        }];
        original.attachments = vec![Attachment {
            id: "file-1".to_owned(),
            kind: AttachmentKind::Link,
            url: "https://example.invalid/spec".to_owned(),
            name: Some("spec".to_owned()),
        }];
        original.recurrence = Some(RecurrenceRule {
            frequency: RecurrenceFrequency::Weekly,
            interval: 1,
            weekdays: vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday],
            month_day: None,
            on_last_day: false,
            calendar: RecurrenceCalendar::Gregorian,
            until: None,
            count: Some(10),
        });
        original.validate().expect("the filled todo is valid");

        let encoded = serde_json::to_string(&original).expect("encode");
        let decoded: Todo = serde_json::from_str(&encoded).expect("decode");

        assert_eq!(decoded.notes.as_deref(), Some("with a note"));
        assert_eq!(decoded.estimated_minutes, Some(60));
        assert_eq!(decoded.sort_order, Some(1.5));
        assert_eq!(decoded.important, Some(true));
        assert_eq!(decoded.urgent, Some(false));
        assert_eq!(decoded.tag_ids, vec!["tag-1".to_owned()]);
        assert_eq!(decoded.subtasks.len(), 1);
        assert!(decoded.subtasks[0].done);
        assert_eq!(decoded.attachments[0].url, "https://example.invalid/spec");
        let recurrence = decoded
            .recurrence
            .expect("recurrence survives the round trip");
        assert_eq!(recurrence.weekdays.len(), 3);
        assert_eq!(recurrence.count, Some(10));
    }

    #[test]
    fn camel_case_is_what_goes_on_the_wire() {
        let mut original = todo();
        original.start_date = Some("2026-07-18".to_owned());
        original.tag_ids = vec!["tag-1".to_owned()];

        let encoded = serde_json::to_string(&original).expect("encode");

        assert!(encoded.contains("\"startDate\":\"2026-07-18\""));
        assert!(encoded.contains("\"tagIds\":[\"tag-1\"]"));
        assert!(!encoded.contains("start_date"));
        // Absent optionals stay off the wire entirely.
        assert!(!encoded.contains("notes"));
    }

    #[test]
    fn an_unknown_field_is_refused() {
        let json = r#"{
            "id": "todo-1",
            "title": "from a newer build",
            "status": "open",
            "createdAt": "2026-07-16T00:00:00Z",
            "completedAt": null,
            "energyLevel": "high"
        }"#;

        assert!(serde_json::from_str::<Todo>(json).is_err());
    }

    #[test]
    fn validation_rejects_the_combinations_that_cannot_be_stored() {
        let mut empty_title = todo();
        empty_title.title = "   ".to_owned();
        assert_eq!(
            empty_title.validate(),
            Err(ContractValidationError::InvalidTitle)
        );

        let mut end_without_start = todo();
        end_without_start.ends_at = Some("2026-07-20T10:00:00Z".to_owned());
        assert_eq!(
            end_without_start.validate(),
            Err(ContractValidationError::InvalidTimeRange)
        );

        let mut zero_estimate = todo();
        zero_estimate.estimated_minutes = Some(0);
        assert_eq!(
            zero_estimate.validate(),
            Err(ContractValidationError::InvalidEstimate)
        );

        let mut duplicate_tags = todo();
        duplicate_tags.tag_ids = vec!["tag-1".to_owned(), "tag-1".to_owned()];
        assert_eq!(
            duplicate_tags.validate(),
            Err(ContractValidationError::DuplicateEntry)
        );

        let mut self_dependency = todo();
        self_dependency.depends_on = vec!["todo-1".to_owned()];
        assert_eq!(
            self_dependency.validate(),
            Err(ContractValidationError::SelfDependency)
        );

        let mut blank_attachment = todo();
        blank_attachment.attachments = vec![Attachment {
            id: "file-1".to_owned(),
            kind: AttachmentKind::File,
            url: "  ".to_owned(),
            name: None,
        }];
        assert_eq!(
            blank_attachment.validate(),
            Err(ContractValidationError::InvalidAttachment)
        );
    }

    #[test]
    fn recurrence_rejects_impossible_rules() {
        let base = RecurrenceRule {
            frequency: RecurrenceFrequency::Monthly,
            interval: 1,
            weekdays: Vec::new(),
            month_day: None,
            on_last_day: false,
            calendar: RecurrenceCalendar::Gregorian,
            until: None,
            count: None,
        };

        let mut no_interval = base.clone();
        no_interval.interval = 0;
        assert_eq!(
            no_interval.validate(),
            Err(ContractValidationError::InvalidRecurrence)
        );

        let mut both_month_positions = base.clone();
        both_month_positions.month_day = Some(15);
        both_month_positions.on_last_day = true;
        assert_eq!(
            both_month_positions.validate(),
            Err(ContractValidationError::InvalidRecurrence)
        );

        let mut impossible_day = base.clone();
        impossible_day.month_day = Some(32);
        assert_eq!(
            impossible_day.validate(),
            Err(ContractValidationError::InvalidRecurrence)
        );

        let mut repeated_weekday = base.clone();
        repeated_weekday.frequency = RecurrenceFrequency::Weekly;
        repeated_weekday.weekdays = vec![Weekday::Monday, Weekday::Monday];
        assert_eq!(
            repeated_weekday.validate(),
            Err(ContractValidationError::DuplicateEntry)
        );

        let mut last_day = base;
        last_day.on_last_day = true;
        last_day
            .validate()
            .expect("the last day of a month is valid");
    }

    #[test]
    fn a_patch_carrying_the_new_fields_is_validated() {
        let patch = TodoPatch {
            subtasks: Some(vec![Subtask {
                id: "step-1".to_owned(),
                title: "  ".to_owned(),
                done: false,
            }]),
            ..TodoPatch::default()
        };

        let operation = TodoSyncOperation {
            operation_id: "operation-1".to_owned(),
            todo_id: "todo-1".to_owned(),
            kind: SyncOperationKind::Upsert,
            occurred_at: "2026-07-16T00:00:00Z".to_owned(),
            patch: Some(patch),
        };

        assert_eq!(
            operation.validate(),
            Err(ContractValidationError::InvalidTitle)
        );
    }
}
