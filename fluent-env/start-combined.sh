#!/bin/bash

# Start Neo4j in the background
neo4j start &

gunicorn -w 2 -b 0.0.0.0:5000 --worker-class gevent --timeout 9000 app:app &

echo "started neo4j and web server"

wait
