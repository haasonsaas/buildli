#!/usr/bin/env python3
"""
Example gRPC client for buildli.
Requires: pip install grpcio grpcio-tools
"""

import sys
import os

# Add the generated proto directory to Python path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'target', 'debug', 'build'))

try:
    import grpc
    print("✓ grpcio is installed")
except ImportError:
    print("✗ Please install grpcio: pip install grpcio grpcio-tools")
    sys.exit(1)

# Simple test without generated code
def test_grpc_connection():
    """Test basic gRPC connectivity"""
    channel = grpc.insecure_channel('localhost:9091')
    
    try:
        # Test channel connectivity
        grpc.channel_ready_future(channel).result(timeout=5)
        print("✓ Successfully connected to gRPC server on localhost:9091")
        return True
    except grpc.FutureTimeoutError:
        print("✗ Could not connect to gRPC server on localhost:9091")
        return False
    finally:
        channel.close()

def main():
    print("Testing buildli gRPC server...")
    
    if test_grpc_connection():
        print("\nThe gRPC server is running and accepting connections!")
        print("\nTo use the full gRPC API, you'll need to:")
        print("1. Generate Python code from the .proto file:")
        print("   python -m grpc_tools.protoc -I../proto --python_out=. --grpc_python_out=. ../proto/buildli.proto")
        print("2. Import and use the generated client code")
    else:
        print("\nMake sure the buildli server is running:")
        print("  buildli serve --port 9090")

if __name__ == "__main__":
    main()