# Issues

- We need some proper error handling, maybe use some of the rust libraries
    like anyhow or smth. Now it's just a mess. 

    The best solution would be to use `thiserror` for library code
    and `anyhow` for application code.

- and there is a lot of magic numbers like the mqtt paths in `mqtt_adapter`

- add configuration from a file

- use redis resistance a sith wont clutter project directory
    https://github.com/fpagliughi/mqtt.rust.redis