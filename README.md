To run DB migration:
```cargo run --bin migrate```

To purge DB (only messages and chat_user):
```cargo run --bin purge```

To disable sqlx logs:
```export RUST_LOG="sqlx=error,info"```
