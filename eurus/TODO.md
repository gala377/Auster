# Things to do now
- setup mongo db
- setup mosquitto authentication with mongodb
- create user for runtime on startup
- add create user option for service
- add redis persistance for mqtt messages.
    https://github.com/fpagliughi/mqtt.rust.redis
- then all of the runtime can be implemented.


## Some things I did not have time for
- runtime task doesn't do anything with the runtime response yet
- runtime task should authorize users # rather mqtt should authorize users, read and write preferences and all
- runtime doesn't do anything in general


## General issues

- dynamic user creation with read, write permissions for channels.
- we need to plan on possible messages.

## General

- when mqtt side of things is finished we can write game state
- after game state comes game logic
- then we can write simple client