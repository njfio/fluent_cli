#!/bin/bash

# Start Neo4j in the background
neo4j start &

screen -d -m flask python /app/app.py

echo "started neo4j and web server"