#!/usr/bin/env python3
"""
Test script for Fluent CLI MCP integration.
This script validates that the MCP server can be started and responds correctly.
"""

import subprocess
import json
import sys
import time
import signal
import os
from typing import Dict, Any, Optional

class MCPTester:
    def __init__(self, fluent_binary: str = "./target/release/fluent"):
        self.fluent_binary = fluent_binary
        self.server_process: Optional[subprocess.Popen] = None

    def start_mcp_server(self) -> subprocess.Popen:
        """Start the MCP server process."""
        print("🚀 Starting Fluent CLI MCP Server...")
        
        # Start the server with STDIO transport
        process = subprocess.Popen(
            [self.fluent_binary, "openai", "mcp", "--stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0
        )
        
        self.server_process = process
        
        # Give the server a moment to start
        time.sleep(2)
        
        if process.poll() is not None:
            stdout, stderr = process.communicate()
            raise RuntimeError(f"MCP server failed to start. Stdout: {stdout}, Stderr: {stderr}")
        
        print("✅ MCP Server started successfully")
        return process

    def send_mcp_request(self, method: str, params: Dict[str, Any] = None) -> Dict[str, Any]:
        """Send an MCP request to the server."""
        if not self.server_process:
            raise RuntimeError("MCP server not started")
        
        request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params or {}
        }
        
        request_json = json.dumps(request) + "\n"
        print(f"📤 Sending request: {method}")
        
        try:
            self.server_process.stdin.write(request_json)
            self.server_process.stdin.flush()
            
            # Read response
            response_line = self.server_process.stdout.readline()
            if not response_line:
                raise RuntimeError("No response from server")
            
            response = json.loads(response_line.strip())
            print(f"📥 Received response for {method}")
            return response
            
        except Exception as e:
            print(f"❌ Error sending request {method}: {e}")
            raise

    def test_server_info(self) -> bool:
        """Test getting server info."""
        try:
            response = self.send_mcp_request("initialize", {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "fluent-cli-test",
                    "version": "1.0.0"
                }
            })
            
            if "result" in response:
                print("✅ Server info test passed")
                print(f"   Server: {response['result'].get('serverInfo', {}).get('name', 'Unknown')}")
                return True
            else:
                print(f"❌ Server info test failed: {response}")
                return False
                
        except Exception as e:
            print(f"❌ Server info test failed with exception: {e}")
            return False

    def test_list_tools(self) -> bool:
        """Test listing available tools."""
        try:
            response = self.send_mcp_request("tools/list")
            
            if "result" in response and "tools" in response["result"]:
                tools = response["result"]["tools"]
                print(f"✅ List tools test passed - found {len(tools)} tools")
                for tool in tools[:3]:  # Show first 3 tools
                    print(f"   - {tool.get('name', 'Unknown')}: {tool.get('description', 'No description')}")
                return True
            else:
                print(f"❌ List tools test failed: {response}")
                return False
                
        except Exception as e:
            print(f"❌ List tools test failed with exception: {e}")
            return False

    def test_call_tool(self) -> bool:
        """Test calling a tool."""
        try:
            response = self.send_mcp_request("tools/call", {
                "name": "list_files",
                "arguments": {"path": "."}
            })
            
            if "result" in response:
                print("✅ Call tool test passed")
                return True
            else:
                print(f"❌ Call tool test failed: {response}")
                return False
                
        except Exception as e:
            print(f"❌ Call tool test failed with exception: {e}")
            return False

    def cleanup(self):
        """Clean up the server process."""
        if self.server_process:
            print("🧹 Cleaning up MCP server...")
            try:
                self.server_process.terminate()
                self.server_process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.server_process.kill()
                self.server_process.wait()
            print("✅ MCP server cleaned up")

    def run_tests(self) -> bool:
        """Run all MCP tests."""
        print("🧪 Starting Fluent CLI MCP Integration Tests")
        print("=" * 50)
        
        try:
            # Start server
            self.start_mcp_server()
            
            # Run tests
            tests = [
                ("Server Info", self.test_server_info),
                ("List Tools", self.test_list_tools),
                ("Call Tool", self.test_call_tool),
            ]
            
            passed = 0
            total = len(tests)
            
            for test_name, test_func in tests:
                print(f"\n🔍 Running test: {test_name}")
                if test_func():
                    passed += 1
                else:
                    print(f"❌ Test failed: {test_name}")
            
            print("\n" + "=" * 50)
            print(f"📊 Test Results: {passed}/{total} tests passed")
            
            if passed == total:
                print("🎉 All tests passed! MCP integration is working correctly.")
                return True
            else:
                print("❌ Some tests failed. MCP integration needs attention.")
                return False
                
        except Exception as e:
            print(f"❌ Test suite failed with exception: {e}")
            return False
        finally:
            self.cleanup()

def main():
    """Main test function."""
    if len(sys.argv) > 1:
        fluent_binary = sys.argv[1]
    else:
        fluent_binary = "./target/release/fluent"
    
    if not os.path.exists(fluent_binary):
        print(f"❌ Fluent binary not found at {fluent_binary}")
        print("   Please build the project first: cargo build --release")
        sys.exit(1)
    
    tester = MCPTester(fluent_binary)
    
    # Handle Ctrl+C gracefully
    def signal_handler(sig, frame):
        print("\n🛑 Test interrupted by user")
        tester.cleanup()
        sys.exit(1)
    
    signal.signal(signal.SIGINT, signal_handler)
    
    success = tester.run_tests()
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
