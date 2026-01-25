import asyncio
import websockets
import json
import random

# Global state to track vehicle status
vehicle_state = {
    "status": "stopped",  # stopped, running, error
    "speed": 0,
    "battery": 100
}

async def handler(websocket):
    client_id = f"{websocket.remote_address}"
    print(f"✅ Client connected: {client_id}")
    
    try:
        # Send initial welcome message
        await websocket.send(json.dumps({
            "type": "welcome",
            "message": "Connected to UGV test server"
        }))
        
        # Send current vehicle state
        await websocket.send(json.dumps({
            "type": "state_update",
            "payload": vehicle_state
        }))
        
        # Simulate sending telemetry data
        async def send_telemetry():
            while True:
                # Only send telemetry if vehicle is running
                if vehicle_state["status"] == "running":
                    data = {
                        "type": "telemetry",
                        "payload": {
                            "speed": random.uniform(5, 10),
                            "battery": max(0, vehicle_state["battery"] - random.uniform(0, 0.1)),
                            "position": {
                                "x": random.uniform(-100, 100),
                                "y": random.uniform(-100, 100)
                            },
                            "heading": random.uniform(0, 360)
                        }
                    }
                    vehicle_state["battery"] = data["payload"]["battery"]
                else:
                    # Send zero speed when stopped
                    data = {
                        "type": "telemetry",
                        "payload": {
                            "speed": 0,
                            "battery": vehicle_state["battery"],
                            "position": {"x": 0, "y": 0},
                            "heading": 0
                        }
                    }
                
                await websocket.send(json.dumps(data))
                await asyncio.sleep(1)
        
        # Start sending telemetry
        telemetry_task = asyncio.create_task(send_telemetry())
        
        # Listen for commands from client
        async for message in websocket:
            print(f"📥 Received from {client_id}: {message}")
            
            try:
                data = json.loads(message)
                command = data.get('payload') or data.get('command')
                
                # Handle different commands
                if command == 'start':
                    vehicle_state["status"] = "running"
                    response = {
                        "type": "command_response",
                        "command": "start",
                        "success": True,
                        "message": "🚗 Vehicle started successfully!"
                    }
                    print(f"✅ Vehicle started by {client_id}")
                    
                elif command == 'stop':
                    vehicle_state["status"] = "stopped"
                    response = {
                        "type": "command_response",
                        "command": "stop",
                        "success": True,
                        "message": "🛑 Vehicle stopped successfully!"
                    }
                    print(f"⏹️ Vehicle stopped by {client_id}")
                    
                elif command == 'reset':
                    vehicle_state["status"] = "stopped"
                    vehicle_state["battery"] = 100
                    response = {
                        "type": "command_response",
                        "command": "reset",
                        "success": True,
                        "message": "🔄 Vehicle reset successfully!"
                    }
                    print(f"🔄 Vehicle reset by {client_id}")
                    
                else:
                    response = {
                        "type": "command_response",
                        "command": command,
                        "success": False,
                        "message": f"❌ Unknown command: {command}"
                    }
                
                # Send response back to client
                await websocket.send(json.dumps(response))
                
                # Also send updated state
                await websocket.send(json.dumps({
                    "type": "state_update",
                    "payload": vehicle_state
                }))
                
            except json.JSONDecodeError:
                print(f"❌ Invalid JSON from {client_id}")
            
    except websockets.exceptions.ConnectionClosed:
        print(f"❌ Client disconnected: {client_id}")
    finally:
        telemetry_task.cancel()

async def main():
    async with websockets.serve(handler, "localhost", 8081):
        print("🚀 WebSocket server started on ws://localhost:8081")
        print("⏳ Waiting for connections...")
        await asyncio.Future()

if __name__ == "__main__":
    asyncio.run(main())