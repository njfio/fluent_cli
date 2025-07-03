from flask import Flask, request, jsonify
import subprocess
import logging
import os
import tempfile
import atexit
import signal
import sys
from pathlib import Path
import shutil

app = Flask(__name__, static_folder='frontend')

# Global list to track temporary files for cleanup
_temp_files = []

def cleanup_temp_files():
    """Clean up all temporary files on exit"""
    for temp_file in _temp_files:
        try:
            if os.path.exists(temp_file):
                os.unlink(temp_file)
                logging.info(f"Cleaned up temporary file: {temp_file}")
        except Exception as e:
            logging.error(f"Failed to clean up {temp_file}: {e}")
    _temp_files.clear()

# Register cleanup handlers
atexit.register(cleanup_temp_files)
signal.signal(signal.SIGTERM, lambda signum, frame: cleanup_temp_files())
signal.signal(signal.SIGINT, lambda signum, frame: cleanup_temp_files())

@app.route('/', methods=['GET'])
def index():
    return app.send_static_file('index.html')

@app.route('/execute', methods=['POST'])
def execute_fluent():
    """Execute fluent command with security validation"""
    try:
        data = request.json
        if not data:
            return jsonify({'error': 'No JSON data provided'}), 400

        # Validate required fields
        if 'engine' not in data:
            return jsonify({'error': 'Engine is required'}), 400

        # Validate engine value
        allowed_engines = ['openai', 'anthropic', 'google', 'cohere', 'mistral']
        if data['engine'] not in allowed_engines:
            return jsonify({'error': f'Invalid engine. Allowed: {allowed_engines}'}), 400

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
        logging.error(f"Subprocess error: {e}")
        return jsonify({'error': str(e)}), 500
    except ValueError as e:
        # Input validation errors
        logging.error(f"Validation error: {e}")
        return jsonify({'error': str(e)}), 400
    except Exception as e:
        # Unexpected errors
        logging.error(f"Unexpected error: {e}")
        return jsonify({'error': 'Internal server error'}), 500
    finally:
        # Clean up any temporary files created in this request
        # Note: Global cleanup happens on exit, but we could add request-specific cleanup here
        pass

# Helper functions to create temporary files with security and cleanup
def create_temp_file(content, extension):
    """Create a temporary file with proper security and cleanup tracking"""
    if not content:
        return None

    # Input validation
    if len(content) > 10 * 1024 * 1024:  # 10MB limit
        raise ValueError("Content too large (max 10MB)")

    # Validate extension
    allowed_extensions = ['.json', '.yaml', '.yml', '.txt']
    if extension not in allowed_extensions:
        raise ValueError(f"Invalid extension. Allowed: {allowed_extensions}")

    try:
        # Create secure temporary file
        with tempfile.NamedTemporaryFile(
            delete=False,
            suffix=extension,
            mode='w',
            encoding='utf-8',
            prefix='fluent_temp_'
        ) as temp:
            temp.write(content)
            temp_path = temp.name

        # Track for cleanup
        _temp_files.append(temp_path)
        logging.info(f"Created temporary file: {temp_path}")

        # Set restrictive permissions (owner read/write only)
        os.chmod(temp_path, 0o600)

        return temp_path

    except Exception as e:
        logging.error(f"Failed to create temporary file: {e}")
        raise

if __name__ == '__main__':
    logging.basicConfig(level=logging.DEBUG)
    logging.debug("Flask app starting...")
    logging.debug(f"Working directory: {os.getcwd()}")
    logging.debug(f"Environment variables: {os.environ}")
    app.run(debug=True, host='0.0.0.0')