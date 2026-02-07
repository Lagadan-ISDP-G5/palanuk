import asyncio
import websockets
import json
import zenoh
from datetime import datetime
import struct

# Global Zenoh session
zenoh_session = None

# Connected WebSocket clients
connected_clients = set()

# Dashboard data storage
dashboard_data = {
    "navigation": {},
    "energy": {},
    "control_state": {
        "loopmode": 0,
        "stop": 0,
        "drivestate": 0
    }
}

class ZenohBridge:
    def __init__(self):
        self.session = None
        
    async def start(self):
        global zenoh_session
        
        conf = zenoh.Config()
        self.session = zenoh.open(conf)
        zenoh_session = self.session
        
        print("🔗 Zenoh session initialized")
        self.subscribe_to_topics()
        
    def subscribe_to_topics(self):
        """Subscribe to data from vehicle"""
        
        self.session.declare_subscriber(
            'palanuk/odd/navigation/position',
            self.handle_navigation_position
        )
        
        self.session.declare_subscriber(
            'palanuk/odd/navigation/heading',
            self.handle_navigation_heading
        )
        
        self.session.declare_subscriber(
            'palanuk/odd/navigation/speed',
            self.handle_navigation_speed
        )
        
        self.session.declare_subscriber(
            'palanuk/odd/energy/battery',
            self.handle_energy_battery
        )
        
        self.session.declare_subscriber(
            'palanuk/odd/drivestate',
            self.handle_drivestate_feedback
        )
        
        print("📡 Subscribed to vehicle data topics")
    
    def handle_navigation_position(self, sample):
        try:
            # FIXED: Convert ZBytes to string
            payload_str = bytes(sample.payload).decode('utf-8')
            data = json.loads(payload_str)
            if 'position' not in dashboard_data['navigation']:
                dashboard_data['navigation']['position'] = {}
            dashboard_data['navigation']['position'] = data
            asyncio.create_task(self.broadcast_to_dashboard('navigation', dashboard_data['navigation']))
        except Exception as e:
            print(f"Error handling position: {e}")
    
    def handle_navigation_heading(self, sample):
        try:
            payload_str = bytes(sample.payload).decode('utf-8')
            data = json.loads(payload_str)
            dashboard_data['navigation']['heading'] = data.get('heading', 0)
            asyncio.create_task(self.broadcast_to_dashboard('navigation', dashboard_data['navigation']))
        except Exception as e:
            print(f"Error handling heading: {e}")
    
    def handle_navigation_speed(self, sample):
        try:
            payload_str = bytes(sample.payload).decode('utf-8')
            data = json.loads(payload_str)
            dashboard_data['navigation']['speed'] = data.get('speed', 0)
            asyncio.create_task(self.broadcast_to_dashboard('navigation', dashboard_data['navigation']))
        except Exception as e:
            print(f"Error handling speed: {e}")
    
    def handle_energy_battery(self, sample):
        try:
            payload_str = bytes(sample.payload).decode('utf-8')
            data = json.loads(payload_str)
            dashboard_data['energy'] = data
            asyncio.create_task(self.broadcast_to_dashboard('energy', dashboard_data['energy']))
        except Exception as e:
            print(f"Error handling battery: {e}")
    
    def handle_drivestate_feedback(self, sample):
        try:
            # FIXED: Convert ZBytes to bytes for struct
            payload_bytes = bytes(sample.payload)
            drivestate = struct.unpack('B', payload_bytes)[0]
            dashboard_data['control_state']['drivestate'] = drivestate
            
            state_names = {0: 'At Rest', 1: 'Forward', 2: 'Reverse'}
            print(f"🚗 Vehicle state: {state_names.get(drivestate, 'Unknown')}")
            
            asyncio.create_task(self.broadcast_to_dashboard('vehicle_state', {
                'status': state_names.get(drivestate, 'Unknown').lower().replace(' ', '_')
            }))
        except Exception as e:
            print(f"Error handling drivestate: {e}")
    
    async def broadcast_to_dashboard(self, component, data):
        """Send data to all connected dashboard clients"""
        if connected_clients:
            message = json.dumps({
                'type': 'component_update',
                'component': component,
                'data': data,
                'timestamp': datetime.now().isoformat()
            })
            
            await asyncio.gather(
                *[client.send(message) for client in connected_clients],
                return_exceptions=True
            )

def publish_control_command(topic_suffix, value, value_type='u8'):
    """Publish control command to Zenoh"""
    if zenoh_session:
        try:
            topic = f'palanuk/odd/{topic_suffix}'
            
            if value_type == 'u8':
                payload = struct.pack('B', int(value))
            elif value_type == 'f64':
                payload = struct.pack('d', float(value))
            else:
                payload = str(value).encode('utf-8')
            
            zenoh_session.put(topic, payload)
            print(f"📤 {topic} = {value}")
            return True
            
        except Exception as e:
            print(f"❌ Error publishing: {e}")
            return False
    return False

def handle_dashboard_command(command):
    """Convert dashboard button commands to Zenoh control messages"""
    print(f"\n📥 Dashboard command: {command}")
    print("-" * 40)
    
    success = False
    
    if command == 'forward':
        success = publish_control_command('drivestate', 1, 'u8')
        publish_control_command('speed', 1.0, 'f64')
        publish_control_command('stop', 0, 'u8')
        
    elif command == 'backward':
        success = publish_control_command('drivestate', 2, 'u8')
        publish_control_command('speed', 1.0, 'f64')
        publish_control_command('stop', 0, 'u8')
        
    elif command == 'left':
        success = publish_control_command('steer/left', 0.8, 'f64')
        publish_control_command('steer/right', 0.2, 'f64')
        publish_control_command('forcepan', 1, 'u8')
        
    elif command == 'right':
        success = publish_control_command('steer/left', 0.2, 'f64')
        publish_control_command('steer/right', 0.8, 'f64')
        publish_control_command('forcepan', 2, 'u8')
        
    elif command == 'stop':
        success = publish_control_command('stop', 1, 'u8')
        publish_control_command('drivestate', 0, 'u8')
        publish_control_command('speed', 0.0, 'f64')
    
    print("-" * 40)
    return success

async def websocket_handler(websocket):
    """Handle WebSocket connections from dashboard"""
    connected_clients.add(websocket)
    print(f"\n✅ Dashboard connected: {websocket.remote_address}")
    
    try:
        await websocket.send(json.dumps({
            'type': 'welcome',
            'message': 'Connected to UGV Demo Bridge'
        }))
        
        async for message in websocket:
            try:
                data = json.loads(message)
                
                if data.get('type') == 'command':
                    command = data.get('payload')
                    success = handle_dashboard_command(command)
                    
                    await websocket.send(json.dumps({
                        'type': 'command_response',
                        'command': command,
                        'success': success,
                        'message': f'✅ "{command}" command sent' if success else '❌ Failed',
                        'timestamp': datetime.now().isoformat()
                    }))
                
                # NEW: Handle control mode changes
                elif data.get('type') == 'control_mode':
                    mode_value = data.get('payload')  # 0 = open, 1 = closed
                    mode_name = 'Closed Loop (Autonomous)' if mode_value == 1 else 'Open Loop (Manual)'
                    
                    print(f"\n🎛️  Control mode change: {mode_name}")
                    success = publish_control_command('loopmode', mode_value, 'u8')
                    
                    await websocket.send(json.dumps({
                        'type': 'control_mode_response',
                        'success': success,
                        'message': f'🎛️ Switched to {mode_name}',
                        'timestamp': datetime.now().isoformat()
                    }))
                    
            except json.JSONDecodeError:
                print(f"❌ Invalid JSON")
                
    except websockets.exceptions.ConnectionClosed:
        print(f"\n❌ Dashboard disconnected")
    finally:
        connected_clients.remove(websocket)

async def main():
    global zenoh_bridge
    zenoh_bridge = ZenohBridge()
    await zenoh_bridge.start()
    
    async with websockets.serve(websocket_handler, "localhost", 8081):
        print("\n" + "=" * 60)
        print("🚀 DEMO BRIDGE READY")
        print("=" * 60)
        print("WebSocket: ws://localhost:8081")
        print("=" * 60)
        
        await asyncio.Future()

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\n👋 Bridge stopped")