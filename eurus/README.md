# Eurus - Auster's backend

Eurus consists of few modules:
- hyper.rs server for starting a runtime and getting access to it;
- runtime which is a tokio task run in the background for each room
witch which you can communicate using mqtt;
- mongodb database holding permissions for clients of mqtt server and
persistent data;
- mosquitto server for mqtt connection;
- mosquitto authorization plugin used to authorize users through mongodb;
- [not yet] redis server providing qos 2 for mqtt queues;

hyper.rs server and runtime are provided by this single rust
project and can be run as such.

mosquitto server, (redis) and mongodb have to be configured and run
separately.

## About security
System was build on mosquitto 1.6.9 and uses
authentication plugin https://github.com/iegomez/mosquitto-go-auth#mongodb using mongodb backend
whichs configuration looks like so:

```
auth_opt_hasher bcrypt
auth_opt_hasher_cost 10

auth_opt_backend mongo
auth_opt_mongo_dbname eurusDB
auth_opt_mongo_users mqtt_users
auth_opt_mongo_acls mqtt_acls
```

When using in production remember to configure mosquitto
and mongodb accordingly.

