# Query Fn

a relatively simple macro which gives you named fields for `Query<(...)>` and `Single<(...)>` in bevy

example:

```rust
#[query_fn]
fn system(foo: Single<(Entity, &Transform), With<Foo>>) {
    println!("{} is at {}", foo.entity, foo.transform.translation);
}
```

expands to

```rust
fn system(foo: Single<_system_0, With<Foo>>) {
    println!("{} is at {}", foo.entity, foo.transform.translation);
}
#[derive(QueryData)]
pub struct _system_0 {
    entity: Entity,
    transform: &'static Transform,
}
```

allowing nicely name fields