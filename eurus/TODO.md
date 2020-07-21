# Issues

- Sending a message that cannot be parsed by serde closes the runtime thread

It is mostly because iterator in `mqtt_adapter` returns `Option<SubMsg>`. Pahu
mqtt returns `Some(None)` in case we disconnected so we can reconenct.
So the problem is when we canno parse the message we return `Some(None)`
which is interpreted as the channel disconnection. It should be changed
to the `Result<SubMsg, RecvError>` and `RecvError` enum.