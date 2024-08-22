#!/bin/bash

# Start Neo4j in the background
neo4j start &

gunicorn -w 2 app:app &

echo "started neo4j and web server"

bash
