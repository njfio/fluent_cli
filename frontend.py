from flask import Flask, request, jsonify
import subprocess
import logging
import os

app = Flask(__name__, static_folder='frontend')

@app.route('/', methods=['GET'])
def index():
    return app.send_static_file('index.html')

@app.route('/execute', methods=['POST'])
def execute_fluent():
    command = request.json['command']
    engine = request.json['engine']  # Get engine from request

    try:
        # Construct the fluent command with the engine
        fluent_command = ["fluent", engine, command]
        output = subprocess.check_output(fluent_command, shell=True).decode('utf-8')
        return jsonify({'output': output})
    except subprocess.CalledProcessError as e:
        return jsonify({'error': str(e)})

if __name__ == '__main__':
    logging.debug("Flask app starting...")
    logging.debug(f"Working directory: {os.getcwd()}")
    logging.debug(f"Environment variables: {os.environ}")
    app.run(debug=True, host='0.0.0.0')