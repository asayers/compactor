<h1 align="center">compactor</h1>

A `Compactor` is like a `Vec`, but it automatically reduces the resolution of
the data as it gets older.

You specify a compaction policy, like this:

```rust
let mut compactor = Compactor::<MyData>::new()
    .keep_for_days(7, Resolution::FiveMinute)
    .keep_for_days(30, Resolution::Hour)
    .keep_for_days(100, Resolution::Day)
    .build()?;
```

and how to compact your data, like this:

```rust
impl Aggregate for MyData {
    fn merge(&mut self, other: MyData) {
        *self = (*self + other) / 2;
    }
}
```

and then you can start pushing data into the compactor.  Initially, data will
be stored at "five-minute" resolution, meaning that pushed values will be merged
into the previous value if they belong to the same five-minute bucket.  After 7
days, the data will be compacted further, down to "one-hour" resolution.  This
means that any values within the same one-hour bucket (up to 12 values) will be
merged into a single value.  Data older than 30 days will be compacted again,
and finally data older than 100 days will be deleted.

## Licence

This software is in the public domain.  See UNLICENSE for details.
