pub use todo_contracts::SyncCursor;

pub fn next_sync_cursor(current: SyncCursor) -> Option<SyncCursor> {
    current.checked_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_increments_without_wrapping() {
        assert_eq!(next_sync_cursor(41), Some(42));
        assert_eq!(next_sync_cursor(SyncCursor::MAX), None);
    }
}
