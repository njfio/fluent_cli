#!/bin/bash

# Start the web server
screen -d -m flask python /app/app.py

echo "started web server"