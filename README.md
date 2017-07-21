# zmq-proxy

A simple command-line wrapper around zmq::proxy.

Example:
$ zmq-proxy ROUTER tcp://*:5559 DEALER tcp://*:5560

(This example implements the msgqueue example from zguide.)

Available socket types: ROUTER, DEALER, XPUB, XSUB.