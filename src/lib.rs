use std::time::SystemTime;
use std::{collections::VecDeque, fmt::Debug};

/// A bucket that stores items with a name, supports history, and enforces limits on items and history.
///
/// # Type Parameters
/// * `T` - The type of items stored in the bucket. Must implement `Clone` and `Debug`.
pub struct RBucket<T: Clone + Debug> {
    /// The name of the bucket.
    pub name: String,
    /// The items currently in the bucket.
    pub items: VecDeque<T>,
    /// The history of polled items and their epochs.
    pub history: Vec<(VecDeque<T>, i64)>,
    /// The maximum number of history entries to keep.
    pub history_limit: i64,
    /// The maximum number of items allowed in the bucket.
    pub items_limit: i64,
}

impl<T: Clone + Debug> RBucket<T> {
    /// Creates a new `RBucket` with the given name and optional limits.
    ///
    /// # Arguments
    /// * `name` - The name of the bucket.
    /// * `history_limit` - Optional limit for the number of history entries (default: 100).
    /// * `items_limit` - Optional limit for the number of items (default: 100).
    pub fn new(name: String, history_limit: Option<i64>, items_limit: Option<i64>) -> Self {
        RBucket {
            name,
            items: VecDeque::new(),
            history: Vec::new(),
            history_limit: history_limit.unwrap_or(100),
            items_limit: items_limit.unwrap_or(100),
        }
    }

    /// Returns an iterator over the items in the bucket.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    /// Undoes the last poll operation, restoring the last polled items to the bucket.
    pub fn undo(&mut self) {
        if let Some((last_items, _)) = self.history.pop() {
            self.items.extend(last_items);
        }
    }

    /// Removes all items from the bucket.
    pub fn clear_items(&mut self) {
        self.items.clear();
    }
    /// Removes all history entries from the bucket.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
    /// Sets the maximum number of history entries.
    pub fn set_history_limit(&mut self, limit: i64) {
        self.history_limit = limit;
    }
    /// Sets the maximum number of items allowed in the bucket.
    pub fn set_items_limit(&mut self, limit: i64) {
        self.items_limit = limit;
    }
    /// Returns true if the history limit has been reached.
    pub fn history_limit_reached(&self) -> bool {
        self.history.len() as i64 >= self.history_limit
    }
    /// Clears history if the history limit is reached. Returns true if history was cleared.
    pub fn history_limit_guard(&mut self) -> bool {
        if self.history_limit_reached() {
            self.history.clear();
            return true;
        }
        false
    }
    /// Returns true if the items limit has been reached.
    pub fn items_limit_reached(&self) -> bool {
        self.items.len() as i64 >= self.items_limit
    }
    /// Clears items if the items limit is reached. Returns true if items were cleared.
    pub fn items_limit_guard(&mut self) -> bool {
        if self.items_limit_reached() {
            self.items.clear();
            return true;
        }
        false
    }
    /// Adds a single item to the bucket, enforcing the items limit.
    pub fn add_item(&mut self, item: T) {
        if !self.items_limit_guard() {
            self.items.push_back(item);
        }
    }
    /// Adds multiple items to the bucket, enforcing the items limit.
    pub fn add_items(&mut self, items: Vec<T>) {
        if !self.items_limit_guard() {
            self.items.append(&mut VecDeque::from(items));
        }
    }

    /// Removes and returns the first item in the bucket, storing it in history with the current epoch.
    pub fn poll(&mut self) -> Option<T> {
        if self.items.is_empty() {
            return None;
        }
        if self.items_limit_reached() {
            self.items_limit_guard();
        }
        if self.history_limit_reached() {
            self.history_limit_guard();
        }
        let i = self.items.pop_front().unwrap();
        let epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.history.push((VecDeque::from(vec![i.clone()]), epoch));
        Some(i)
    }
}

/// Implements deep cloning for `RBucket`.
impl<T: Clone + Debug> Clone for RBucket<T> {
    fn clone(&self) -> Self {
        RBucket {
            name: self.name.clone(),
            items: self.items.clone(),
            history: self.history.clone(),
            history_limit: self.history_limit,
            items_limit: self.items_limit,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        *self = source.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.add_item(1);
        bucket.add_item(2);
        assert_eq!(bucket.poll(), Some(1));
        assert_eq!(bucket.poll(), Some(2));
        assert_eq!(bucket.poll(), None);
        assert_eq!(bucket.history.len(), 2);
        assert_eq!(bucket.history[0].0, VecDeque::from(vec![1]));
        assert_eq!(bucket.history[1].0, VecDeque::from(vec![2]));
        // Check that the epoch is stored correctly
        let epoch1 = bucket.history[0].1;
        let epoch2 = bucket.history[1].1;
        assert!(epoch1 <= epoch2, "Epochs should be in order");
    }

    #[test]
    fn threaded() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        let bucket = Arc::new(Mutex::new(RBucket::new("test".into() , None, None)));
        let mut handles = vec![];

        for i in 0..10 {
            let bucket_clone = Arc::clone(&bucket);
            let handle = thread::spawn(move || {
                let mut bucket = bucket_clone.lock().unwrap();
                bucket.add_item(i);
                bucket.poll()
            });
            handles.push(handle);
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        let bucket = bucket.lock().unwrap();
        // Check that all items were added and polled
        assert_eq!(bucket.history.len(), 10);
        let mut found: Vec<i32> = bucket.history.iter().map(|(v, _)| v[0]).collect();
        found.sort();
        for i in 0..10 {
            assert_eq!(found[i], i as i32);
        }
        // Check that the epochs are in order
        for i in 1..bucket.history.len() {
            assert!(
                bucket.history[i - 1].1 <= bucket.history[i].1,
                "Epochs should be in order"
            );
        }
    }

    #[test]
    fn clone_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.add_item(1);
        bucket.add_item(2);
        let cloned_bucket = bucket.clone();
        assert_eq!(cloned_bucket.name, "test");
        assert_eq!(cloned_bucket.items.len(), 2);
        assert_eq!(cloned_bucket.history.len(), 0); // History should not be cloned
    }
    #[test]
    fn add_items_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.add_items(vec![1, 2, 3]);
        assert_eq!(bucket.items.len(), 3);
        assert_eq!(bucket.items[0], 1);
        assert_eq!(bucket.items[1], 2);
        assert_eq!(bucket.items[2], 3);
    }
    #[test]
    fn iter_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.add_items(vec![1, 2, 3]);
        let items: Vec<_> = bucket.iter().collect();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], &1);
        assert_eq!(items[1], &2);
        assert_eq!(items[2], &3);
    }
    #[test]
    fn undo_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.add_item(1);
        bucket.add_item(2);
        bucket.poll(); // Polls 1
        bucket.poll(); // Polls 2
        assert_eq!(bucket.items.len(), 0);
        bucket.undo(); // Undo last poll
        assert_eq!(bucket.items.len(), 1);
        assert_eq!(bucket.items[0], 2); // Should have restored the last polled item
    }
    #[test]
    fn clear_items_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.add_items(vec![1, 2, 3]);
        assert_eq!(bucket.items.len(), 3);
        bucket.clear_items();
        assert_eq!(bucket.items.len(), 0);
    }
    #[test]
    fn clear_history_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.add_item(1);
        bucket.poll(); // Polls 1
        assert_eq!(bucket.history.len(), 1);
        bucket.clear_history();
        assert_eq!(bucket.history.len(), 0);
    }
    #[test]
    fn history_limit_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.set_history_limit(2);
        bucket.add_item(1);
        bucket.poll(); // Polls 1
        bucket.add_item(2);
        bucket.poll(); // Polls 2
        assert_eq!(bucket.history.len(), 2);
        bucket.add_item(3);
        bucket.poll(); // Polls 3, should clear history
        assert_eq!(bucket.history.len(), 1); // Only the last item should remain
    }
    #[test]
    fn items_limit_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.set_items_limit(2);
        bucket.add_item(1);
        bucket.add_item(2);
        assert_eq!(bucket.items.len(), 2);
        bucket.add_item(3); // This should clear the items
        assert_eq!(bucket.items.len(), 0); // Items should be cleared
    }
    #[test]
    fn history_limit_guard_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.set_history_limit(2);
        bucket.add_item(1);
        bucket.poll(); // Polls 1
        bucket.add_item(2);
        bucket.poll(); // Polls 2
        assert!(bucket.history_limit_guard()); // Should clear history
        assert_eq!(bucket.history.len(), 0); // History should be cleared
    }
    #[test]
    fn items_limit_guard_test() {
        let mut bucket = RBucket::new("test".into(), None, None);
        bucket.set_items_limit(2);
        bucket.add_item(1);
        bucket.add_item(2);
        assert!(bucket.items_limit_guard()); // Should clear items
        assert_eq!(bucket.items.len(), 0); // Items should be cleared
    }
}
