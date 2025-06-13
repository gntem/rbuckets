
use std::time::SystemTime;
use std::{collections::VecDeque, fmt::Debug};

pub struct RBucket<T: Clone + Debug> {
    pub name: String,
    pub items: VecDeque<T>,
    pub history: Vec<(VecDeque<T>, i64)>,
}

impl<T: Clone + Debug> RBucket<T> {
    pub fn new(name: String) -> Self {
        RBucket {
            name,
            items: VecDeque::new(),
            history: Vec::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    pub fn undo(&mut self) {
        if let Some((last_items, _)) = self.history.pop() {
            self.items.extend(last_items);
        }
    }

    pub fn add_item(&mut self, item: T) {
        self.items.push_back(item);
    }

    pub fn add_items(&mut self, items: Vec<T>) {
        self.items.append(&mut VecDeque::from(items));
    }

    pub fn poll(&mut self) -> Option<T> {
        if self.items.is_empty() {
            return None;
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

impl<T: Clone + Debug> Clone for RBucket<T> {
    fn clone(&self) -> Self {
        RBucket {
            name: self.name.clone(),
            items: self.items.clone(),
            history: self.history.clone(),
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
        let mut bucket = RBucket::new("test".into());
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
        use std::thread;
        use std::sync::{Arc, Mutex};
        let bucket = Arc::new(Mutex::new(RBucket::new("test".into())));
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
            assert!(bucket.history[i - 1].1 <= bucket.history[i].1, "Epochs should be in order");
        }
    }

    #[test]
    fn clone_test() {
        let mut bucket = RBucket::new("test".into());
        bucket.add_item(1);
        bucket.add_item(2);
        let cloned_bucket = bucket.clone();
        assert_eq!(cloned_bucket.name, "test");
        assert_eq!(cloned_bucket.items.len(), 2);
        assert_eq!(cloned_bucket.history.len(), 0); // History should not be cloned
    }
    #[test]
    fn add_items_test() {
        let mut bucket = RBucket::new("test".into());
        bucket.add_items(vec![1, 2, 3]);
        assert_eq!(bucket.items.len(), 3);
        assert_eq!(bucket.items[0], 1);
        assert_eq!(bucket.items[1], 2);
        assert_eq!(bucket.items[2], 3);
    }
    #[test]
    fn iter_test() {
        let mut bucket = RBucket::new("test".into());
        bucket.add_items(vec![1, 2, 3]);
        let items: Vec<_> = bucket.iter().collect();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], &1);
        assert_eq!(items[1], &2);
        assert_eq!(items[2], &3);
    }
}
