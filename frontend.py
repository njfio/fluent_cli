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

    fluent_command = ["fluent"]

    # Add engine
    fluent_command.append(data['engine'])

    # Add request (optional)
    if data.get('request'):
        fluent_command.append(data['request'])

    # Add options
    add_option(fluent_command, '-c', config_file)
    for override in data.get('override', '').split():
        add_option(fluent_command, '-o', override)
    add_option(fluent_command, '-a', data.get('additionalContextFile'))
    add_flag(fluent_command, '--upsert', data.get('upsert'))
    add_option(fluent_command, '-i', data.get('input'))
    add_option(fluent_command, '-t', data.get('metadata'))
    add_option(fluent_command, '-l', data.get('uploadImageFile'))
    add_option(fluent_command, '-d', data.get('downloadMedia'))
    add_flag(fluent_command, '-p', data.get('parseCode'))
    add_flag(fluent_command, '-x', data.get('executeOutput'))
    add_flag(fluent_command, '-m', data.get('markdown'))
    add_option(fluent_command, '--generate-cypher', data.get('generateCypher'))

    # Handle pipeline command
    if pipeline_file:
        fluent_command.extend(['pipeline', '--file', pipeline_file])
        add_option(fluent_command, '--input', data.get('pipelineInput'))
        add_flag(fluent_command, '--json-output', data.get('jsonOutput'))
        add_option(fluent_command, '--run-id', data.get('runId'))
        add_flag(fluent_command, '--force-fresh', data.get('forceFresh'))

    try:
        logging.debug(f"Executing command: {fluent_command}")
        output = subprocess.check_output(fluent_command).decode('utf-8')
        return jsonify({'output': output})
    except subprocess.CalledProcessError as e:
        return jsonify({'error': str(e)})

# Helper functions to add options/flags to the command list
def add_option(command_list, flag, value):
    if value:
        command_list.extend([flag, value])

def add_flag(command_list, flag, is_set):
    if is_set:
        command_list.append(flag)

# Helper function to create temporary files
def create_temp_file(content, extension):
    if content:
        with tempfile.NamedTemporaryFile(delete=False, suffix=extension) as temp:
            temp.write(content.encode('utf-8'))
            return temp.name
    return None

if __name__ == '__main__':
    logging.debug("Flask app starting...")
    logging.debug(f"Working directory: {os.getcwd()}")
    logging.debug(f"Environment variables: {os.environ}")
    app.run(debug=True, host='0.0.0.0')