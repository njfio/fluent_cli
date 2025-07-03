"""
Secure Flask frontend for fluent_cli with enhanced security measures.
Addresses security vulnerabilities identified in code review analysis.
"""

from flask import Flask, request, jsonify
import subprocess
import logging
import os
import tempfile
import atexit
import signal
import sys
import time
import shlex
import re
from pathlib import Path
from functools import wraps
from collections import defaultdict
import threading

app = Flask(__name__, static_folder='frontend')

# Security configuration
MAX_REQUEST_SIZE = 10 * 1024 * 1024  # 10MB
MAX_REQUESTS_PER_MINUTE = 30
COMMAND_TIMEOUT = 60  # seconds
ALLOWED_ENGINES = ['openai', 'anthropic', 'google', 'cohere', 'mistral']
ALLOWED_EXTENSIONS = ['.json', '.yaml', '.yml', '.txt']

# Rate limiting storage
request_counts = defaultdict(list)
rate_limit_lock = threading.Lock()

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

def rate_limit(max_requests=MAX_REQUESTS_PER_MINUTE):
    """Rate limiting decorator"""
    def decorator(f):
        @wraps(f)
        def decorated_function(*args, **kwargs):
            client_ip = request.environ.get('HTTP_X_FORWARDED_FOR', request.remote_addr)
            current_time = time.time()
            
            with rate_limit_lock:
                # Clean old requests (older than 1 minute)
                request_counts[client_ip] = [
                    req_time for req_time in request_counts[client_ip]
                    if current_time - req_time < 60
                ]
                
                # Check rate limit
                if len(request_counts[client_ip]) >= max_requests:
                    return jsonify({'error': 'Rate limit exceeded. Try again later.'}), 429
                
                # Add current request
                request_counts[client_ip].append(current_time)
            
            return f(*args, **kwargs)
        return decorated_function
    return decorator

def validate_input(data):
    """Comprehensive input validation"""
    if not data:
        raise ValueError('No JSON data provided')
    
    # Check request size
    if len(str(data)) > MAX_REQUEST_SIZE:
        raise ValueError('Request too large')
    
    # Validate required fields
    if 'engine' not in data:
        raise ValueError('Engine is required')
    
    # Validate engine
    if data['engine'] not in ALLOWED_ENGINES:
        raise ValueError(f'Invalid engine. Allowed: {ALLOWED_ENGINES}')
    
    # Validate string inputs for injection attacks
    dangerous_patterns = [
        r'[;&|`$()]',  # Shell metacharacters
        r'\.\./',      # Path traversal
        r'<script',    # XSS
        r'exec\s*\(',  # Code execution
    ]
    
    for key, value in data.items():
        if isinstance(value, str):
            for pattern in dangerous_patterns:
                if re.search(pattern, value, re.IGNORECASE):
                    raise ValueError(f'Invalid characters in {key}')
    
    return True

def sanitize_command_args(args):
    """Sanitize command arguments to prevent injection"""
    sanitized = []
    for arg in args:
        if isinstance(arg, str):
            # Remove dangerous characters and limit length
            sanitized_arg = re.sub(r'[;&|`$()]', '', arg)[:1000]
            sanitized.append(sanitized_arg)
        else:
            sanitized.append(str(arg)[:1000])
    return sanitized

def execute_command_safely(command_args):
    """Execute command with security sandboxing"""
    try:
        # Sanitize arguments
        safe_args = sanitize_command_args(command_args)
        
        # Log the command for debugging (sanitized version)
        logging.debug(f"Executing command: {safe_args}")
        
        # Execute with timeout and limited environment
        env = {
            'PATH': '/usr/local/bin:/usr/bin:/bin',
            'HOME': '/tmp',
            'TMPDIR': '/tmp'
        }
        
        result = subprocess.run(
            safe_args,
            capture_output=True,
            text=True,
            timeout=COMMAND_TIMEOUT,
            env=env,
            cwd='/tmp',  # Run in safe directory
            check=False  # Don't raise on non-zero exit
        )
        
        # Sanitize output to prevent information leakage
        output = result.stdout
        error = result.stderr
        
        # Remove sensitive information from error messages
        if error:
            error = re.sub(r'/[^\s]*fluent[^\s]*', '[REDACTED_PATH]', error)
            error = re.sub(r'api[_-]?key[^\s]*', '[REDACTED_KEY]', error, flags=re.IGNORECASE)
        
        if result.returncode != 0:
            logging.error(f"Command failed with code {result.returncode}: {error}")
            return None, f"Command execution failed: {error[:500]}"  # Limit error message length
        
        return output, None
        
    except subprocess.TimeoutExpired:
        logging.error("Command execution timed out")
        return None, "Command execution timed out"
    except Exception as e:
        logging.error(f"Command execution error: {e}")
        return None, "Internal server error"

def create_temp_file(content, extension):
    """Create a temporary file with proper security and cleanup tracking"""
    if not content:
        return None
    
    # Input validation
    if len(content) > MAX_REQUEST_SIZE:
        raise ValueError("Content too large")
    
    # Validate extension
    if extension not in ALLOWED_EXTENSIONS:
        raise ValueError(f"Invalid extension. Allowed: {ALLOWED_EXTENSIONS}")
    
    # Validate content for dangerous patterns
    dangerous_patterns = [
        r'<script',
        r'javascript:',
        r'data:',
        r'vbscript:',
    ]
    
    for pattern in dangerous_patterns:
        if re.search(pattern, content, re.IGNORECASE):
            raise ValueError("Content contains dangerous patterns")
    
    try:
        # Create secure temporary file
        with tempfile.NamedTemporaryFile(
            delete=False,
            suffix=extension,
            mode='w',
            encoding='utf-8',
            prefix='fluent_secure_',
            dir='/tmp'  # Use system temp directory
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

@app.route('/', methods=['GET'])
def index():
    return app.send_static_file('index.html')

@app.route('/execute', methods=['POST'])
@rate_limit()
def execute_fluent():
    """Execute fluent command with enhanced security validation"""
    try:
        data = request.json
        
        # Comprehensive input validation
        validate_input(data)
        
        # Handle config and pipeline files
        config_file = create_temp_file(data.get('config'), '.json')
        pipeline_file = create_temp_file(data.get('pipelineFile'), '.yaml')
        
        # Start building the fluent command
        fluent_command = ["fluent"]
        
        # Add the engine
        fluent_command.append(data['engine'])
        
        # Add the request (optional, with length limit)
        if data.get('request'):
            request_text = str(data['request'])[:5000]  # Limit request length
            fluent_command.append(request_text)
        
        # Add options with validation
        if config_file:
            fluent_command.extend(['-c', config_file])
        
        # Handle overrides with validation
        overrides = data.get('override', '')
        if overrides:
            for override in overrides.split()[:10]:  # Limit number of overrides
                if override and len(override) < 100:  # Limit override length
                    fluent_command.extend(['-o', override])
        
        # Add other options with validation
        if data.get('additionalContextFile'):
            context_file = str(data.get('additionalContextFile'))[:500]
            fluent_command.extend(['-a', context_file])
        
        if data.get('upsert'):
            fluent_command.append('--upsert')
        
        if data.get('input'):
            input_text = str(data.get('input'))[:1000]
            fluent_command.extend(['-i', input_text])
        
        if data.get('metadata'):
            metadata = str(data.get('metadata'))[:500]
            fluent_command.extend(['-t', metadata])
        
        # Handle pipeline command
        if pipeline_file:
            fluent_command.extend(['pipeline', '--file', pipeline_file])
            
            if data.get('pipelineInput'):
                pipeline_input = str(data.get('pipelineInput'))[:1000]
                fluent_command.extend(['--input', pipeline_input])
            
            if data.get('jsonOutput'):
                fluent_command.append('--json-output')
            
            if data.get('runId'):
                run_id = str(data.get('runId'))[:100]
                fluent_command.extend(['--run-id', run_id])
            
            if data.get('forceFresh'):
                fluent_command.append('--force-fresh')
        
        # Execute command safely
        output, error = execute_command_safely(fluent_command)
        
        if error:
            return jsonify({'error': error}), 500
        
        return jsonify({'output': output})
        
    except ValueError as e:
        logging.error(f"Validation error: {e}")
        return jsonify({'error': str(e)}), 400
    except Exception as e:
        logging.error(f"Unexpected error: {e}")
        return jsonify({'error': 'Internal server error'}), 500
    finally:
        # Request-specific cleanup could be added here
        pass

if __name__ == '__main__':
    # Configure secure logging
    logging.basicConfig(
        level=logging.INFO,  # Changed from DEBUG for production
        format='%(asctime)s - %(levelname)s - %(message)s',
        handlers=[
            logging.FileHandler('/tmp/fluent_frontend.log'),
            logging.StreamHandler()
        ]
    )
    
    logging.info("Secure Flask app starting...")
    
    # Run with security settings
    app.run(
        debug=False,  # Disable debug mode for security
        host='127.0.0.1',  # Bind to localhost only
        port=5000,
        threaded=True
    )
