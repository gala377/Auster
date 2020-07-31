# Issues

## General issues

- runtime subscribes to the runtime-channel so when it sends a value
    there it also receives it. Channels should be splitted, one for
    receiving values and another one for broadcast.
- dynamic user creation with read, write permissions for channels.
- we need to plan on possible messages. 

## MQTT things

- mongodb or mysql integration
- mqtt authorization
- creating temporatry users for the runtime and the game
- use redis resistance a sith wont clutter project directory
    https://github.com/fpagliughi/mqtt.rust.redis


## General

- when mqtt side of things is finished we can write game state
- after game state comes game logic
- then we can write simple client