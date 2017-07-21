# zmq-proxy

A simple command-line wrapper around `zmq::proxy` and basic sockets.

Examples (adapted from zguide):

- hwclient:
```
$ yes hello | zmq req connect tcp://localhost:5555
```

- hwserver:
```
$ yes world | zmq rep bind tcp://*:5555
```

- wuserver (simplified):
```
$ yes "zipcode temperature relhumidity" | zmq pub bind tcp://*:5556
```

- wuclient (simplified):
```
$ zmq sub connect tcp://localhost:5556 zipcode
```

- msgqueue:
```
$ zmq proxy router tcp://*:5559 dealer tcp://*:5560
```

Available socket types for proxy: `router`, `dealer`, `xpub`, `xsub`.