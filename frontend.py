from flask import Flask, request, jsonify
import subprocess

app = Flask(__name__, static_url_path='', static_folder='frontend')

@app.route('/', methods=['GET'])
def index():
    return app.send_static_file('front_end_index.html')

@app.route('/execute', methods=['POST'])
def execute_fluent():
    command = request.json['command']
    try:
        output = subprocess.check_output(["fluent", command], shell=True).decode('utf-8')
        return jsonify({'output': output})
    except subprocess.CalledProcessError as e:
        return jsonify({'error': str(e)})

if __name__ == '__main__':
    app.run(debug=True, host='0.0.0.0')