# Issues

- Sending a message that cannot be parsed by serde closes the runtime thread

It is mostly because iterator in `mqtt_adapter` returns `Option<SubMsg>`. Pahu
mqtt returns `Some(None)` in case we disconnected so we can reconenct.
So the problem is when we canno parse the message we return `Some(None)`
which is interpreted as the channel disconnection. It should be changed
to the `Result<SubMsg, RecvError>` and `RecvError` enum.

- We need some proper error handling, maybe use some of the rust libraries
    like anyhow or smth. Now it's just a mess. 

    The best solution would be to use `thiserror` for library code
    and `anyhow` for application code.

- We need some proper logging. Printlns just wont cut it.

- and there is a lot of magic numbers like the mqtt paths in `mqtt_adapter`
