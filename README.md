# rbuckets

A generic Rust bucket structure with history and item limits.

## Features
- Store items of any type (e.g., strings, numbers)
- Enforce limits on the number of items and history entries
- Undo, clear, and poll items with epoch tracking

## Example Usage

```rust
use rbuckets::RBucket;

fn main() {
    // Create a new bucket for fruit, with default limits
    let mut fruit_bucket = RBucket::new("fruit".to_string(), None, None);

    // Add apples and bananas
    fruit_bucket.add_item("apple");
    fruit_bucket.add_item("banana");

    // Add multiple items at once
    fruit_bucket.add_items(vec!["apple", "banana"]);

    // Iterate over items
    for fruit in fruit_bucket.iter() {
        println!("Fruit: {}", fruit);
    }

    // Poll (remove) the first item
    if let Some(fruit) = fruit_bucket.poll() {
        println!("Polled: {}", fruit);
    }

    // Undo the last poll
    fruit_bucket.undo();
}
```

## License
MIT
