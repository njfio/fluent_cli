#!/bin/bash

# Start Neo4j in the background
neo4j start &

gunicorn -w 2 app:app 0.0.0.0:5000 &

echo "started neo4j and web server"

bash
