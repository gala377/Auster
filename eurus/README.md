# About security
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

