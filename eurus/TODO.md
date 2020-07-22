# Issues

- We need some proper error handling, maybe use some of the rust libraries
    like anyhow or smth. Now it's just a mess. 

    The best solution would be to use `thiserror` for library code
    and `anyhow` for application code.

- We need some proper logging. Printlns just wont cut it.

- and there is a lot of magic numbers like the mqtt paths in `mqtt_adapter`

- add configuration from a file