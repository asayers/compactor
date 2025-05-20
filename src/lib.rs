/*!
A `Compactor` is like a `Vec`, but it automatically reduces the resolution of
the data as it gets older.

You specify a compaction policy, like this:

```rust
# use compactor::{Compactor, policy::PolicyError, Resolution};
# fn foo() -> Result<(), PolicyError> {
# struct MyData(usize);
let mut compactor = Compactor::<MyData>::new()
    .keep_for_days(7, Resolution::FiveMinute)
    .keep_for_days(30, Resolution::Hour)
    .keep_for_days(100, Resolution::Day)
    .build()?;
# Ok(())
# }
```

You specify how to compact your data, like this:

```rust
# use compactor::Aggregate;
# #[derive(Copy, Clone)]
# struct MyData(usize);
# impl std::ops::Add for MyData { type Output = MyData; fn add(self, other: MyData) -> MyData { MyData(self.0 + other.0) } }
# impl std::ops::Div<usize> for MyData { type Output = MyData; fn div(self, other: usize) -> MyData { MyData(self.0 / other) } }
impl Aggregate for MyData {
    fn merge(&mut self, other: MyData) {
        *self = (*self + other) / 2;
    }
}
```

...and then you can start pushing data into the compactor.

In this example, data will initially be stored at "five-minute" resolution,
meaning that pushed values will be merged into the previous value if they belong
to the same five-minute bucket.

After 7 days, the data will be compacted further, down to "one-hour" resolution.
This means that any values within the same one-hour bucket will be merged into
a single value.  Data older than 30 days will be compacted again.  Finally, data
older than 100 days will be deleted.

*/

pub mod aggregate;
mod compactor;
mod data;
pub mod datetime;
pub mod policy;

pub use crate::aggregate::Aggregate;
pub use crate::compactor::{Compactor, CompactorBuilder};
pub use crate::datetime::{Date, Resolution, Time};
