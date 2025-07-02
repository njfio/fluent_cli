from flask import Flask, request, jsonify
import subprocess
import logging
import os
import tempfile

app = Flask(__name__, static_folder='frontend')

@app.route('/', methods=['GET'])
def index():
    return app.send_static_file('index.html')

@app.route('/execute', methods=['POST'])
def execute_fluent():
    data = request.json

    # Handle config and pipeline files
    config_file = create_temp_file(data.get('config'), '.json')
    pipeline_file = create_temp_file(data.get('pipelineFile'), '.yaml')

    # Start building the fluent command
    fluent_command = ["fluent"]

    # Add the engine
    fluent_command.append(data['engine'])

    # Add the request (optional)
    if data.get('request'):
        fluent_command.append(data['request'])

    # Add the options, checking if they have values
    if config_file:
        fluent_command.extend(['-c', config_file])

    for override in data.get('override', '').split():
        if override:
            fluent_command.extend(['-o', override])

    if data.get('additionalContextFile'):
        fluent_command.extend(['-a', data.get('additionalContextFile')])

    if data.get('upsert'):
        fluent_command.append('--upsert')

    if data.get('input'):
        fluent_command.extend(['-i', data.get('input')])

    if data.get('metadata'):
        fluent_command.extend(['-t', data.get('metadata')])

    if data.get('uploadImageFile'):
        fluent_command.extend(['-l', data.get('uploadImageFile')])

    if data.get('downloadMedia'):
        fluent_command.extend(['-d', data.get('downloadMedia')])

    if data.get('parseCode'):
        fluent_command.append('-p')

    if data.get('executeOutput'):
        fluent_command.append('-x')

    if data.get('markdown'):
        fluent_command.append('-m')

    if data.get('generateCypher'):
        fluent_command.extend(['--generate-cypher', data.get('generateCypher')])

    # Handle the pipeline command
    if pipeline_file:
        fluent_command.extend(['pipeline', '--file', pipeline_file])

        if data.get('pipelineInput'):
            fluent_command.extend(['--input', data.get('pipelineInput')])

        if data.get('jsonOutput'):
            fluent_command.append('--json-output')

        if data.get('runId'):
            fluent_command.extend(['--run-id', data.get('runId')])

        if data.get('forceFresh'):
            fluent_command.append('--force-fresh')

    try:
        # Log the command for debugging
        logging.debug(f"Executing command: {fluent_command}")
        # Execute the fluent command and capture the output
        output = subprocess.check_output(fluent_command).decode('utf-8')
        # Return the output as a JSON response
        return jsonify({'output': output})
    except subprocess.CalledProcessError as e:
        # If there is an error, return the error message as a JSON response
        return jsonify({'error': str(e)})

# Helper functions to create temporary files
def create_temp_file(content, extension):
    if content:
        with tempfile.NamedTemporaryFile(delete=False, suffix=extension) as temp:
            temp.write(content.encode('utf-8'))
            return temp.name
    return None

if __name__ == '__main__':
    logging.basicConfig(level=logging.DEBUG)
    logging.debug("Flask app starting...")
    logging.debug(f"Working directory: {os.getcwd()}")
    logging.debug(f"Environment variables: {os.environ}")
    app.run(debug=True, host='0.0.0.0')