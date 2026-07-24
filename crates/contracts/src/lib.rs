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
///
/// Public for the same reason as the title limit: notes are typed into an input
/// too, and the view layer stops that input at this number rather than at a
/// figure of its own.
pub const MAX_NOTES_CHARS: usize = 20_000;
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
    /// When the task was archived: "done long enough that it need not be seen
    /// any more".
    ///
    /// A field of its own rather than a third `TodoStatus` variant, because
    /// archiving and completing are orthogonal. A status can only say one thing
    /// at a time, so folding the two together would lose "it was completed, and
    /// then archived" — and every reader of the status (the open count, the
    /// completed set, the statistics still to come) would have to be redefined
    /// to keep meaning what it means today.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub archived_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub due_date: Option<String>,
    /// Reminder points on the task, each firing at its own moment (提醒通知/08).
    ///
    /// Replaces the single `reminder_at` a task carried before multi-level
    /// reminders: a lone reminder becomes one entry here with its instant
    /// unchanged, so the delivery key that instant rode on is unchanged too, and
    /// the store's column migration folds an old single value in on read. Defaults
    /// empty like the other list-valued fields, so a payload from a build that
    /// never knew the field reads back with no reminders.
    #[serde(default)]
    pub reminders: Vec<Reminder>,
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
    /// Scheduled dates (`YYYY-MM-DD`) this task has been checked in on.
    ///
    /// A daily or weekly recurring task is shown as a habit (任务管理/09): checking
    /// it off records the scheduled date it stood on here and advances the single
    /// row to its next occurrence, rather than leaving a completed sibling behind.
    /// The streak is a pure function of this set over the schedule — no drifting
    /// counter is stored anywhere. Empty for every task that is not a checked-in
    /// habit, so it defaults like the other list-valued fields and a payload from
    /// a build that never knew it reads back empty.
    #[serde(default)]
    pub check_ins: Vec<String>,
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

        validate_reminders(&self.reminders)?;
        validate_id_list(&self.tag_ids)?;
        validate_subtasks(&self.subtasks)?;
        validate_attachments(&self.attachments)?;
        validate_id_list(&self.depends_on)?;
        if self.depends_on.contains(&self.id) {
            return Err(ContractValidationError::SelfDependency);
        }

        // Check-in dates are civil dates rather than ids, but the same three
        // faults the id lists guard against apply to them too — too many, a
        // blank one, or the same day recorded twice — so the same check fits.
        validate_id_list(&self.check_ins)?;

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

/// One reminder point on a task.
///
/// A task may carry several (提醒通知/08), each firing at its own moment. The pair
/// of fields is what lets the global "fixed time" switch (提醒通知/07) read one
/// stored value two ways without a second copy of it: `at` is the absolute
/// instant, `offset` is where the device sat when the reminder was set, and the
/// wall-clock time the user chose is the two together. With the switch off (the
/// default) a reminder floats — it keeps that wall clock as the device travels;
/// with it on a reminder fires at `at` exactly. A relative "N before the due
/// date" reminder is a later, separate shape (登记为未来便利层) that would need a
/// payload-carrying variant; the absolute point here does not, so the reminder
/// list stays a plain struct with no `#[serde(flatten)]` or enum payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct Reminder {
    /// The instant the reminder is set for, as an ISO 8601 UTC instant — the same
    /// value the single `reminder_at` used to carry, kept verbatim through the
    /// migration so a migrated reminder keeps the delivery key it rode on.
    pub at: String,
    /// The device's offset from UTC in seconds east when the reminder was set.
    ///
    /// Kept so the wall clock behind `at` can be recovered: the wall clock is `at`
    /// read at this offset. Floating delivery fires at that wall clock under the
    /// device's *current* offset; fixed-time delivery ignores it and fires at
    /// `at`. Within the ±14h a real zone can sit from UTC it fits an `i32` with
    /// room to spare.
    pub offset: i32,
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

        // An end date and a count are both stopping conditions, and RFC 5545
        // 3.3.10 forbids writing both: the .ics export (数据与同步/08) can keep
        // only one and keeps the end date, so a rule carrying both would repeat
        // one length in the app and another in an exported calendar. Refusing the
        // pair here — the one chokepoint every stored or synced rule passes
        // through — is what keeps the app, the engine and an export in agreement.
        if self.until.is_some() && self.count.is_some() {
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

/// Reads and writes a patch field that may be cleared, keeping `null` apart
/// from "not there".
///
/// Serde reads a nested option the way it reads a flat one — `null` becomes the
/// outer `None`, which is the very shape a missing key already has — so the two
/// meanings collapse before anything downstream can tell them apart. Reading the
/// inner option and wrapping whatever it produced is what keeps them separate:
/// `deserialize` only ever runs when the key was there, so its answer is always
/// `Some`, and `Some(None)` is a key that was there and said `null`.
mod present {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<T, S>(value: &Option<Option<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        // `skip_serializing_if` keeps an unmentioned field off the wire, so what
        // arrives here is a mention: a value, or the `null` that clears it.
        value.serialize(serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Some)
    }
}

/// The fields one sync operation changes.
///
/// An absent field means "unchanged"; a present one replaces the stored value,
/// list-valued fields included (a tag set is sent whole, not as a diff).
///
/// Nullable fields are `Option<Option<T>>` rather than `Option<T>` so that
/// "leave it alone" and "clear it" are different values instead of the same one:
/// the outer option is whether the patch mentions the field, the inner one is
/// what it sets it to. Without the distinction every optional field could be
/// filled in and never emptied again — a clear left home as `null`, arrived as
/// "unchanged", and the other devices went on showing the old value.
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
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>", optional = nullable)]
    pub due_date: Option<Option<String>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>", optional = nullable)]
    pub completed_at: Option<Option<String>>,
    /// Nested like every other nullable field, and for the sharpest instance of
    /// the reason: restoring an archived task *is* clearing this field, so a
    /// flat option would send that restore as "unchanged" and the task would
    /// stay archived on every other device.
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>", optional = nullable)]
    pub archived_at: Option<Option<String>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>", optional = nullable)]
    pub notes: Option<Option<String>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>", optional = nullable)]
    pub start_date: Option<Option<String>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>", optional = nullable)]
    pub starts_at: Option<Option<String>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>", optional = nullable)]
    pub ends_at: Option<Option<String>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<u32>", optional = nullable)]
    pub estimated_minutes: Option<Option<u32>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<RecurrenceRule>", optional = nullable)]
    pub recurrence: Option<Option<RecurrenceRule>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<String>", optional = nullable)]
    pub list_id: Option<Option<String>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<bool>", optional = nullable)]
    pub important: Option<Option<bool>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<bool>", optional = nullable)]
    pub urgent: Option<Option<bool>>,
    #[serde(default, with = "present", skip_serializing_if = "Option::is_none")]
    #[ts(as = "Option<f64>", optional = nullable)]
    pub sort_order: Option<Option<f64>>,
    /// The whole reminder list, sent as one value like the tag set rather than as
    /// a diff: absent leaves the reminders alone, an empty list clears them. This
    /// is the shape the single `reminder_at` three-state option gave way to — a
    /// list field replaces wholesale, so "no reminders" is `[]`, not the `null`
    /// the old option carried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub reminders: Option<Vec<Reminder>>,
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
    /// The whole set of scheduled dates a habit has been checked in on, sent as
    /// one value like the tag set rather than as a diff.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub check_ins: Option<Vec<String>>,
}

impl TodoPatch {
    pub fn validate(&self) -> Result<(), ContractValidationError> {
        if let Some(title) = &self.title {
            validate_title(title)?;
        }

        if let Some(Some(notes)) = &self.notes {
            if notes.chars().count() > MAX_NOTES_CHARS {
                return Err(ContractValidationError::InvalidNotes);
            }
        }

        if self.estimated_minutes == Some(Some(0)) {
            return Err(ContractValidationError::InvalidEstimate);
        }

        if let Some(Some(recurrence)) = &self.recurrence {
            recurrence.validate()?;
        }

        if let Some(Some(list_id)) = &self.list_id {
            validate_id(list_id)?;
        }

        if let Some(reminders) = &self.reminders {
            validate_reminders(reminders)?;
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
        if let Some(check_ins) = &self.check_ins {
            validate_id_list(check_ins)?;
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

/// Reminders share the list ceiling every other list field is held to, so a task
/// can never carry a set the database would refuse. The instant itself is left
/// unchecked, exactly as the single `reminder_at` was: a value no reader can time
/// is skipped when the reminder is weighed, not refused at the door — the same
/// leniency the calendar export and the scheduler already apply to a stored
/// instant.
fn validate_reminders(reminders: &[Reminder]) -> Result<(), ContractValidationError> {
    if reminders.len() > MAX_LIST_ENTRIES {
        return Err(ContractValidationError::TooManyEntries);
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

/// What the server can say about itself: whether it is serving, how the sync
/// requests it has served have been going, and what the log has been unable to
/// read.
///
/// `HealthResponse` above stays what it is — the liveness answer the app asks
/// for before syncing, which must not grow or change shape for a monitoring
/// need. This is the operator's view, served separately.
#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HealthReport {
    pub status: HealthStatus,
    pub service: String,
    pub time: String,
    pub uptime_seconds: u64,
    pub sync: SyncHealth,
    pub log: LogHealth,
    /// The concerns that are open right now, oldest first. Empty is the answer
    /// an operator wants to see.
    pub alerts: Vec<HealthAlert>,
}

/// How well the server is doing what it is for.
///
/// `Degraded` is "worth looking at, still serving"; `Unhealthy` is "the answers
/// it gives cannot be trusted", which is the one a probe should act on.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, TS, Type)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Degraded,
    Unhealthy,
}

/// How sync requests have been going, over a window and over the whole run.
///
/// Latencies are whole request timings, taken around the service call: the
/// durable commit and the wait for the single connection are inside them,
/// because a device waiting for its operations to be accepted waits for both.
#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SyncHealth {
    /// How far back the windowed numbers below reach.
    pub window_seconds: u64,
    pub requests: u64,
    /// Requests the server could not serve — the log refused them. These are
    /// the server's own failures, unlike `rejections`.
    pub failures: u64,
    /// Requests refused because what they carried was not acceptable. A device
    /// sending nonsense is not the server being unwell.
    pub rejections: u64,
    pub average_millis: f64,
    pub slowest_millis: f64,
    /// The longest a request waited for the log's single connection before it
    /// could start. This is what separates "the disk is slow" from "requests
    /// are queueing behind each other".
    pub lock_wait_slowest_millis: f64,
    pub requests_total: u64,
    pub failures_total: u64,
}

/// What the log itself has to say.
#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LogHealth {
    /// Whether the log answered when it was last looked at.
    pub reachable: bool,
    /// Whether the look found the connection in use and did not wait for it.
    /// Busy is not unwell: it is what a server under load looks like.
    pub busy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub latest_revision: Option<SyncCursor>,
    /// Rows no reading could make sense of, so devices never receive them.
    pub unreadable_rows: RowFaults,
    /// Rows served without the fields this build has no name for — what a
    /// rolled back binary looks like from the log's side.
    pub partly_read_rows: RowFaults,
}

/// A count of rows with something wrong with them, told so it can be acted on:
/// how many distinct rows, when the first and the newest turned up, and what
/// the newest one was.
#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RowFaults {
    /// Distinct rows met since the process started, counted once each however
    /// often they are read.
    pub rows: u64,
    /// How many times such a row has been read, which is the number a log line
    /// per read would have produced.
    pub reads: u64,
    /// Distinct rows first met inside the window — the answer to "is this
    /// still spreading", as opposed to "how much of it is there".
    pub new_in_window: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub first_seen_at: Option<String>,
    /// When the most recently discovered distinct row turned up. A number that
    /// keeps moving is a fault that is still spreading.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub newest_seen_at: Option<String>,
    /// The most recently discovered row, in one line: which revision, which
    /// field, and what was wrong with it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub newest: Option<String>,
}

/// One open concern.
#[derive(Clone, Debug, Serialize, TS, Type)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct HealthAlert {
    pub name: String,
    pub severity: HealthStatus,
    /// When it started, so an operator can tell a fresh problem from one that
    /// has been open all day.
    pub since: String,
    pub detail: String,
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
            archived_at: None,
            due_date: None,
            reminders: Vec::new(),
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
            check_ins: Vec::new(),
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
        assert!(decoded.reminders.is_empty());
        assert!(decoded.tag_ids.is_empty());
        assert!(decoded.subtasks.is_empty());
        assert!(decoded.attachments.is_empty());
        assert!(decoded.depends_on.is_empty());
        assert!(decoded.check_ins.is_empty());
        assert_eq!(decoded.important, None);
        assert!(decoded.recurrence.is_none());
        assert!(decoded.archived_at.is_none());
        decoded.validate().expect("a v1 payload stays valid");
    }

    #[test]
    fn archiving_and_completing_are_carried_separately() {
        // The pair a third status variant could not express: completed at one
        // instant, archived at a later one, with both facts still readable.
        let mut archived = todo();
        archived.status = TodoStatus::Completed;
        archived.completed_at = Some("2026-07-16T10:00:00Z".to_owned());
        archived.archived_at = Some("2026-07-23T00:00:05Z".to_owned());
        archived.validate().expect("an archived todo is valid");

        let encoded = serde_json::to_string(&archived).expect("encode");
        assert!(encoded.contains("\"archivedAt\":\"2026-07-23T00:00:05Z\""));
        let decoded: Todo = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded.completed_at.as_deref(), Some("2026-07-16T10:00:00Z"));
        assert_eq!(decoded.archived_at.as_deref(), Some("2026-07-23T00:00:05Z"));
    }

    #[test]
    fn restoring_a_todo_travels_as_a_cleared_archive_stamp() {
        // What "restore" puts on the wire. Were `archived_at` a flat option the
        // `null` would arrive as "unchanged" and the task would stay archived
        // everywhere else — the one failure this field's shape exists to stop.
        let raw = r#"{"status":"open","archivedAt":null,"completedAt":null}"#;
        let decoded: TodoPatch = serde_json::from_str(raw).expect("decode a restore");
        assert_eq!(decoded.archived_at, Some(None));

        let encoded = serde_json::to_string(&decoded).expect("encode");
        assert!(encoded.contains("\"archivedAt\":null"));

        let again: TodoPatch = serde_json::from_str(&encoded).expect("decode again");
        assert_eq!(again.archived_at, Some(None));

        // Archiving is the same field carrying a value, and a patch that says
        // nothing about it still means "leave it alone".
        let archiving: TodoPatch =
            serde_json::from_str(r#"{"archivedAt":"2026-07-23T00:00:05Z"}"#).expect("decode");
        assert_eq!(
            archiving.archived_at,
            Some(Some("2026-07-23T00:00:05Z".to_owned()))
        );
        let untouched: TodoPatch = serde_json::from_str(r#"{"title":"unrelated"}"#).expect("decode");
        assert_eq!(untouched.archived_at, None);
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
        original.check_ins = vec!["2026-07-20".to_owned(), "2026-07-21".to_owned()];
        original.reminders = vec![
            Reminder {
                at: "2026-07-20T09:00:00Z".to_owned(),
                offset: 8 * 3_600,
            },
            Reminder {
                at: "2026-07-20T08:00:00Z".to_owned(),
                offset: 8 * 3_600,
            },
        ];
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
        assert_eq!(
            decoded.check_ins,
            vec!["2026-07-20".to_owned(), "2026-07-21".to_owned()]
        );
        assert_eq!(decoded.reminders.len(), 2);
        assert_eq!(decoded.reminders[0].at, "2026-07-20T09:00:00Z");
        assert_eq!(decoded.reminders[0].offset, 8 * 3_600);
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
    fn recurrence_refuses_an_end_date_and_a_count_at_once() {
        // RFC 5545 3.3.10 forbids UNTIL and COUNT together, and the .ics export
        // can write only one of the two (数据与同步/08 keeps the end date), so a
        // rule carrying both would repeat one length in the app and another in an
        // exported calendar. The single stopping condition is enforced here, at
        // the one chokepoint every stored or synced rule passes through — either
        // one alone, or neither, stays valid.
        let base = RecurrenceRule {
            frequency: RecurrenceFrequency::Daily,
            interval: 1,
            weekdays: Vec::new(),
            month_day: None,
            on_last_day: false,
            calendar: RecurrenceCalendar::Gregorian,
            until: None,
            count: None,
        };

        let mut both = base.clone();
        both.until = Some("2026-12-31".to_owned());
        both.count = Some(10);
        assert_eq!(
            both.validate(),
            Err(ContractValidationError::InvalidRecurrence)
        );

        let mut until_only = base.clone();
        until_only.until = Some("2026-12-31".to_owned());
        until_only
            .validate()
            .expect("an end date on its own is a valid stop");

        let mut count_only = base.clone();
        count_only.count = Some(10);
        count_only
            .validate()
            .expect("a count on its own is a valid stop");

        base.validate()
            .expect("a rule with no stopping condition repeats for ever, which is valid");
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

    #[test]
    fn a_patch_tells_clearing_a_field_from_not_mentioning_it() {
        let cleared: TodoPatch = serde_json::from_str(
            r#"{"dueDate":null,"notes":null,"startDate":null,
                "startsAt":null,"endsAt":null,"estimatedMinutes":null,"reminders":[]}"#,
        )
        .expect("decode a patch that clears fields");

        // Mentioned, and set to nothing.
        assert_eq!(cleared.due_date, Some(None));
        // A reminder list clears by carrying an empty list, not the `null` a
        // three-state option used: present-and-empty is "no reminders".
        assert_eq!(cleared.reminders, Some(Vec::new()));
        assert_eq!(cleared.notes, Some(None));
        assert_eq!(cleared.start_date, Some(None));
        assert_eq!(cleared.starts_at, Some(None));
        assert_eq!(cleared.ends_at, Some(None));
        assert_eq!(cleared.estimated_minutes, Some(None));

        let untouched: TodoPatch =
            serde_json::from_str(r#"{"title":"still here"}"#).expect("decode a patch that does not");
        assert_eq!(untouched.due_date, None);
        assert_eq!(untouched.notes, None);
        assert_eq!(untouched.estimated_minutes, None);
    }

    #[test]
    fn a_cleared_field_survives_the_round_trip_a_stored_patch_makes() {
        // The server stores a patch by re-encoding what it decoded, so a clear
        // that does not survive this trip reaches no other device: it leaves as
        // `null`, is stored as nothing, and comes back as "unchanged".
        let raw = r#"{"title":"clear my dates","dueDate":null,"estimatedMinutes":null}"#;
        let decoded: TodoPatch = serde_json::from_str(raw).expect("decode");
        let encoded = serde_json::to_string(&decoded).expect("encode");

        assert!(encoded.contains("\"dueDate\":null"));
        assert!(encoded.contains("\"estimatedMinutes\":null"));
        // A field nobody mentioned still stays off the wire entirely.
        assert!(!encoded.contains("notes"));

        let again: TodoPatch = serde_json::from_str(&encoded).expect("decode again");
        assert_eq!(again.due_date, Some(None));
        assert_eq!(again.estimated_minutes, Some(None));
        assert_eq!(again.notes, None);
    }

    #[test]
    fn a_patch_setting_a_value_is_unaffected_by_the_clearing_shape() {
        let patch: TodoPatch = serde_json::from_str(
            r#"{"dueDate":"2026-08-01","estimatedMinutes":30,"important":true}"#,
        )
        .expect("decode");

        assert_eq!(patch.due_date, Some(Some("2026-08-01".to_owned())));
        assert_eq!(patch.estimated_minutes, Some(Some(30)));
        assert_eq!(patch.important, Some(Some(true)));
        patch.validate().expect("a filled patch is valid");

        let zero_estimate = TodoPatch {
            estimated_minutes: Some(Some(0)),
            ..TodoPatch::default()
        };
        assert_eq!(
            zero_estimate.validate(),
            Err(ContractValidationError::InvalidEstimate)
        );
        // Clearing an estimate is not setting it to zero.
        let cleared_estimate = TodoPatch {
            estimated_minutes: Some(None),
            ..TodoPatch::default()
        };
        cleared_estimate
            .validate()
            .expect("clearing an estimate is valid");
    }
}
