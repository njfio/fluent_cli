#!/usr/bin/env python3
"""
Advanced CLI Testing Scenarios for Fluent CLI

This script tests complex usage scenarios and edge cases for the Fluent CLI.
"""

import subprocess
import tempfile
import os
import json
import yaml
import sys
from pathlib import Path

class CLITestRunner:
    """Test runner for Fluent CLI scenarios"""

    def __init__(self):
        self.temp_dir = tempfile.mkdtemp()
        self.test_files = {}
        print(f"Using temporary directory: {self.temp_dir}")

    def create_test_file(self, filename, content):
        """Create a test file in the temporary directory"""
        filepath = os.path.join(self.temp_dir, filename)
        with open(filepath, 'w') as f:
            f.write(content)
        self.test_files[filename] = filepath
        return filepath

    def run_command(self, args, expect_success=True):
        """Run a fluent CLI command and return the result"""
        cmd = ['fluent'] + args
        try:
            result = subprocess.run(
                cmd,
                cwd=self.temp_dir,
                capture_output=True,
                text=True,
                timeout=30
            )
            if expect_success and result.returncode != 0:
                print(f"❌ Command failed: {' '.join(cmd)}")
                print(f"STDOUT: {result.stdout}")
                print(f"STDERR: {result.stderr}")
                return result
            return result
        except subprocess.TimeoutExpired:
            print(f"⏰ Command timed out: {' '.join(cmd)}")
            return None
        except Exception as e:
            print(f"💥 Command failed with exception: {' '.join(cmd)} - {e}")
            return None

    def cleanup(self):
        """Clean up temporary files"""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

def test_global_options():
    """Test global CLI options"""
    print("📋 Testing Global Options")

    runner = CLITestRunner()

    # Test help options
    result = runner.run_command(['--help'])
    assert result and result.returncode == 0, "Help command should succeed"
    assert 'fluent' in result.stdout, "Help should contain 'fluent'"

    result = runner.run_command(['-h'])
    assert result and result.returncode == 0, "Short help should succeed"

    # Test version options
    result = runner.run_command(['--version'])
    assert result and result.returncode == 0, "Version command should succeed"

    result = runner.run_command(['-V'])
    assert result and result.returncode == 0, "Short version should succeed"

    # Test config options
    config_content = {
        'engines': [{
            'name': 'test-engine',
            'engine': 'openai',
            'connection': {
                'protocol': 'https',
                'hostname': 'api.openai.com',
                'port': 443,
                'request_path': '/v1/chat/completions'
            },
            'parameters': {
                'model': 'gpt-3.5-turbo'
            }
        }]
    }

    config_file = runner.create_test_file('test_config.yaml', yaml.dump(config_content))
    result = runner.run_command(['--config', config_file, '--help'])
    assert result and result.returncode == 0, "Config option should work"

    result = runner.run_command(['-c', config_file, '--help'])
    assert result and result.returncode == 0, "Short config option should work"

    runner.cleanup()
    print("✅ Global options tests passed")

def test_pipeline_scenarios():
    """Test pipeline command scenarios"""
    print("📋 Testing Pipeline Scenarios")

    runner = CLITestRunner()

    # Create test pipeline
    pipeline_content = {
        'name': 'test_pipeline',
        'steps': [{
            'name': 'test_step',
            'engine': 'test_engine',
            'request': 'Hello, world!'
        }]
    }

    pipeline_file = runner.create_test_file('test_pipeline.yaml', yaml.dump(pipeline_content))

    # Create test config
    config_content = {
        'engines': [{
            'name': 'test_engine',
            'engine': 'openai',
            'connection': {
                'protocol': 'https',
                'hostname': 'api.openai.com',
                'port': 443,
                'request_path': '/v1/chat/completions'
            },
            'parameters': {}
        }]
    }

    config_file = runner.create_test_file('test_config.yaml', yaml.dump(config_content))

    # Test pipeline help
    result = runner.run_command(['pipeline', '--help'])
    assert result and result.returncode == 0, "Pipeline help should succeed"

    # Test pipeline with required file
    result = runner.run_command(['pipeline', '--file', pipeline_file, '--config', config_file, '--dry-run'])
    assert result and result.returncode == 0, "Pipeline dry-run should complete without errors"

    # Test pipeline with all options
    result = runner.run_command([
        'pipeline',
        '--file', pipeline_file,
        '--config', config_file,
        '--input', 'test input',
        '--variables', 'key1=value1',
        '--variables', 'key2=value2',
        '--force-fresh',
        '--run-id', 'test-run-123',
        '--dry-run',
        '--json'
    ])
    assert result and result.returncode == 0, "Pipeline with all options should complete without errors"
    runner.cleanup()
    print("✅ Pipeline scenarios tests passed")

def test_agent_scenarios():
    """Test agent command scenarios"""
    print("📋 Testing Agent Scenarios")

    runner = CLITestRunner()

    # Test agent help
    result = runner.run_command(['agent', '--help'])
    assert result and result.returncode == 0, "Agent help should succeed"

    # Test agent with goal
    result = runner.run_command([
        'agent',
        '--goal', 'Create a simple function',
        '--max-iterations', '5',
        '--reflection',
        '--dry-run'
    ])
    # Should at least parse correctly

    # Create test goal file
    goal_content = {
        'goal_description': 'Create a simple function',
        'max_iterations': 5,
        'success_criteria': ['Function compiles without errors']
    }

    # Write as TOML
    goal_toml = '''goal_description = "Create a simple function"
max_iterations = 5
success_criteria = ["Function compiles without errors"]
'''

    goal_file = runner.create_test_file('test_goal.toml', goal_toml)

    # Test agent with goal file
    result = runner.run_command([
        'agent',
        '--goal-file', goal_file,
        '--model', 'gpt-4o',
        '--gen-retries', '2',
        '--min-html-size', '1000',
        '--dry-run'
    ])
    # Should at least parse correctly

    runner.cleanup()
    print("✅ Agent scenarios tests passed")

def test_mcp_scenarios():
    """Test MCP command scenarios"""
    print("📋 Testing MCP Scenarios")

    runner = CLITestRunner()

    # Test MCP help
    result = runner.run_command(['mcp', '--help'])
    assert result and result.returncode == 0, "MCP help should succeed"

    # Test MCP subcommands help
    result = runner.run_command(['mcp', 'server', '--help'])
    assert result and result.returncode == 0, "MCP server help should succeed"

    result = runner.run_command(['mcp', 'client', '--help'])
    assert result and result.returncode == 0, "MCP client help should succeed"

    runner.cleanup()
    print("✅ MCP scenarios tests passed")

def test_error_scenarios():
    """Test error handling scenarios"""
    print("📋 Testing Error Scenarios")

    runner = CLITestRunner()

    # Test invalid command
    result = runner.run_command(['invalid-command'], expect_success=False)
    assert result and result.returncode != 0, "Invalid command should fail"

    # Test missing required arguments
    result = runner.run_command(['pipeline'], expect_success=False)
    assert result and result.returncode != 0, "Pipeline without --file should fail"

    # Test invalid subcommand
    result = runner.run_command(['pipeline', 'invalid-subcommand'], expect_success=False)
    assert result and result.returncode != 0, "Invalid subcommand should fail"

    runner.cleanup()
    print("✅ Error scenarios tests passed")

def test_complex_combinations():
    """Test complex command combinations"""
    print("📋 Testing Complex Combinations")

    runner = CLITestRunner()

    # Create test config
    config_content = {
        'engines': [{
            'name': 'test-engine',
            'engine': 'openai',
            'connection': {
                'protocol': 'https',
                'hostname': 'api.openai.com',
                'port': 443,
                'request_path': '/v1/chat/completions'
            },
            'parameters': {
                'model': 'gpt-3.5-turbo'
            }
        }]
    }

    config_file = runner.create_test_file('test_config.yaml', yaml.dump(config_content))

    # Test multiple global options
    result = runner.run_command(['--config', config_file, '--help'])
    assert result and result.returncode == 0, "Multiple global options should work"

    # Test nested subcommands
    result = runner.run_command(['tools', 'list', '--json'])
    # Should at least parse correctly

    # Test all major commands help
    commands = [
        ['pipeline', '--help'],
        ['agent', '--help'],
        ['mcp', '--help'],
        ['neo4j', '--help'],
        ['tools', '--help'],
        ['engine', '--help']
    ]

    for cmd in commands:
        result = runner.run_command(cmd)
        assert result and result.returncode == 0, f"Help for {' '.join(cmd)} should succeed"

    runner.cleanup()
    print("✅ Complex combinations tests passed")

def test_tools_scenarios():
    """Test tools command scenarios"""
    print("📋 Testing Tools Scenarios")

    runner = CLITestRunner()

    # Test tools help
    result = runner.run_command(['tools', '--help'])
    assert result and result.returncode == 0, "Tools help should succeed"

    # Test tools list with all options
    result = runner.run_command(['tools', 'list', '--category', 'file', '--search', 'read', '--json', '--available', '--detailed'])
    # Should at least parse correctly

    # Test tools describe with all options
    result = runner.run_command(['tools', 'describe', 'read_file', '--json', '--schema', '--examples'])
    # Should at least parse correctly

    # Test tools exec with options
    result = runner.run_command(['tools', 'exec', 'read_file', '--json-output'])
    # Should at least parse correctly

    # Test tools categories with json
    result = runner.run_command(['tools', 'categories', '--json'])
    # Should at least parse correctly

    runner.cleanup()
    print("✅ Tools scenarios tests passed")

def test_engine_scenarios():
    """Test engine command scenarios"""
    print("📋 Testing Engine Scenarios")

    runner = CLITestRunner()

    # Test engine help
    result = runner.run_command(['engine', '--help'])
    assert result and result.returncode == 0, "Engine help should succeed"

    # Test engine list with json
    result = runner.run_command(['engine', 'list', '--json'])
    # Should at least parse correctly

    # Test engine test (will fail without valid config, but should parse)
    result = runner.run_command(['engine', 'test', 'nonexistent-engine'], expect_success=False)
    # Parsing should work, but execution will fail

    runner.cleanup()
    print("✅ Engine scenarios tests passed")

def main():
    """Run all test scenarios"""
    print("🧪 Fluent CLI Advanced Scenario Tests")
    print("=====================================")

    try:
        test_global_options()
        test_pipeline_scenarios()
        test_agent_scenarios()
        test_mcp_scenarios()
        test_error_scenarios()
        test_complex_combinations()
        test_tools_scenarios()
        test_engine_scenarios()

        print("\n🎉 All advanced scenario tests passed!")
        return 0
    except Exception as e:
        print(f"\n💥 Tests failed with exception: {e}")
        import traceback
        traceback.print_exc()
        return 1

if __name__ == '__main__':
    sys.exit(main())
