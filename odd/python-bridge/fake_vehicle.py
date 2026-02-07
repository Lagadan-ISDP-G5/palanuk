import zenoh
import struct
import time
import json

print("🚗 Starting Fake Vehicle Simulator...")

session = zenoh.open(zenoh.Config())

vehicle_state = {
    'loopmode': 0,  # 0 = open (manual), 1 = closed (autonomous)
    'stop': 0,
    'steer_left': 0.5,
    'steer_right': 0.5,
    'speed': 0.0,
    'drivestate': 0,
    'forcepan': 0,
    'position_x': 0.0,
    'position_y': 0.0,
    'heading': 0.0,
    'actual_speed': 0.0,
    'autonomous_active': False
}

# NEW: Handle loop mode changes
def on_loopmode(sample):
    try:
        payload_bytes = bytes(sample.payload)
        value = struct.unpack('B', payload_bytes)[0]
        vehicle_state['loopmode'] = value
        vehicle_state['autonomous_active'] = (value == 1)
        
        mode_name = 'CLOSED LOOP (Autonomous)' if value == 1 else 'OPEN LOOP (Manual)'
        print(f"\n{'='*60}")
        print(f"🎛️  CONTROL MODE CHANGED: {mode_name}")
        print(f"{'='*60}\n")
        
        if value == 1:
            print("🤖 Autonomous navigation activated - following line")
            # Simulate autonomous behavior
            vehicle_state['speed'] = 0.8  # Auto speed
            vehicle_state['drivestate'] = 1  # Forward
        else:
            print("🎮 Manual control enabled - waiting for commands")
            # Stop when switching to manual
            vehicle_state['speed'] = 0.0
            vehicle_state['drivestate'] = 0
            
    except Exception as e:
        print(f"Error in on_loopmode: {e}")

def on_stop(sample):
    try:
        payload_bytes = bytes(sample.payload)
        value = struct.unpack('B', payload_bytes)[0]
        vehicle_state['stop'] = value
        if value == 1:
            print("🛑 EMERGENCY STOP ACTIVATED!")
            vehicle_state['drivestate'] = 0
            vehicle_state['speed'] = 0.0
            vehicle_state['actual_speed'] = 0.0
    except Exception as e:
        print(f"Error in on_stop: {e}")

def on_steer_left(sample):
    try:
        if vehicle_state['loopmode'] == 1:
            print("⚠️  Steering command ignored - in autonomous mode")
            return
        payload_bytes = bytes(sample.payload)
        value = struct.unpack('d', payload_bytes)[0]
        vehicle_state['steer_left'] = value
        print(f"🔄 Steer Left: {value:.2f}")
    except Exception as e:
        print(f"Error in on_steer_left: {e}")

def on_steer_right(sample):
    try:
        if vehicle_state['loopmode'] == 1:
            print("⚠️  Steering command ignored - in autonomous mode")
            return
        payload_bytes = bytes(sample.payload)
        value = struct.unpack('d', payload_bytes)[0]
        vehicle_state['steer_right'] = value
        print(f"🔄 Steer Right: {value:.2f}")
    except Exception as e:
        print(f"Error in on_steer_right: {e}")

def on_speed(sample):
    try:
        if vehicle_state['loopmode'] == 1:
            print("⚠️  Speed command ignored - in autonomous mode")
            return
        payload_bytes = bytes(sample.payload)
        value = struct.unpack('d', payload_bytes)[0]
        vehicle_state['speed'] = value
        print(f"⚡ Speed Command: {value:.2f} m/s")
    except Exception as e:
        print(f"Error in on_speed: {e}")

def on_drivestate(sample):
    try:
        if vehicle_state['loopmode'] == 1:
            print("⚠️  Drive state command ignored - in autonomous mode")
            return
        payload_bytes = bytes(sample.payload)
        value = struct.unpack('B', payload_bytes)[0]
        vehicle_state['drivestate'] = value
        states = {0: 'At Rest', 1: 'Forward', 2: 'Reverse'}
        print(f"🚗 Drive State: {states.get(value, 'Unknown')}")
    except Exception as e:
        print(f"Error in on_drivestate: {e}")

def on_forcepan(sample):
    try:
        payload_bytes = bytes(sample.payload)
        value = struct.unpack('B', payload_bytes)[0]
        vehicle_state['forcepan'] = value
        pans = {0: 'Center', 1: 'Reference Left', 2: 'Reference Right'}
        print(f"📐 Force Pan: {pans.get(value, 'Unknown')}")
    except Exception as e:
        print(f"Error in on_forcepan: {e}")

# Subscribe
session.declare_subscriber('palanuk/odd/loopmode', on_loopmode)
session.declare_subscriber('palanuk/odd/stop', on_stop)
session.declare_subscriber('palanuk/odd/steer/left', on_steer_left)
session.declare_subscriber('palanuk/odd/steer/right', on_steer_right)
session.declare_subscriber('palanuk/odd/speed', on_speed)
session.declare_subscriber('palanuk/odd/drivestate', on_drivestate)
session.declare_subscriber('palanuk/odd/forcepan', on_forcepan)

print("✅ Subscribed to all control topics")
print("📤 Publishing vehicle telemetry...\n")

# Simulation with autonomous behavior
def simulate_vehicle():
    autonomous_path_angle = 0  # For simulating line following
    
    while True:
        # Autonomous behavior
        if vehicle_state['loopmode'] == 1 and vehicle_state['stop'] == 0:
            # Simulate following a curved path
            autonomous_path_angle += 0.5
            vehicle_state['heading'] = (autonomous_path_angle % 360)
            vehicle_state['drivestate'] = 1
            vehicle_state['actual_speed'] = 0.8
            
            # Simulate slight steering corrections
            correction = 0.5 + 0.1 * (autonomous_path_angle % 10) / 10
            vehicle_state['steer_left'] = 0.5 + correction
            vehicle_state['steer_right'] = 0.5 - correction
        
        # Manual control behavior
        elif vehicle_state['loopmode'] == 0:
            if vehicle_state['stop'] == 0:
                target_speed = vehicle_state['speed']
                
                if vehicle_state['drivestate'] == 1:
                    vehicle_state['actual_speed'] = min(vehicle_state['actual_speed'] + 0.1, target_speed)
                elif vehicle_state['drivestate'] == 2:
                    vehicle_state['actual_speed'] = max(vehicle_state['actual_speed'] - 0.1, -target_speed)
                else:
                    vehicle_state['actual_speed'] *= 0.9
                    if abs(vehicle_state['actual_speed']) < 0.01:
                        vehicle_state['actual_speed'] = 0.0
                        
                steering_diff = vehicle_state['steer_right'] - vehicle_state['steer_left']
                vehicle_state['heading'] += steering_diff * 2.0
                vehicle_state['heading'] %= 360
            else:
                vehicle_state['actual_speed'] = 0.0
        
        # Update position
        import math
        heading_rad = math.radians(vehicle_state['heading'])
        vehicle_state['position_x'] += vehicle_state['actual_speed'] * math.cos(heading_rad) * 0.5
        vehicle_state['position_y'] += vehicle_state['actual_speed'] * math.sin(heading_rad) * 0.5
        
        battery_level = max(50, 100 - time.time() % 50)
        
        # Publish telemetry
        session.put('palanuk/odd/navigation/position', json.dumps({
            'x': round(vehicle_state['position_x'], 2),
            'y': round(vehicle_state['position_y'], 2)
        }))
        
        session.put('palanuk/odd/navigation/heading', json.dumps({
            'heading': round(vehicle_state['heading'], 2)
        }))
        
        session.put('palanuk/odd/navigation/speed', json.dumps({
            'speed': round(vehicle_state['actual_speed'], 2)
        }))
        
        session.put('palanuk/odd/energy/battery', json.dumps({
            'level': round(battery_level, 1),
            'voltage': 12.4,
            'current': -2.3 if vehicle_state['actual_speed'] > 0 else 0,
            'temperature': 32.5
        }))
        
        session.put('palanuk/odd/drivestate', struct.pack('B', vehicle_state['drivestate']))
        
        # Status print
        if int(time.time() * 2) % 10 == 0:
            mode = "AUTO" if vehicle_state['loopmode'] == 1 else "MAN"
            states = {0: 'Rest', 1: 'Fwd', 2: 'Rev'}
            print(f"[{mode}] State: {states[vehicle_state['drivestate']]:<4} | "
                  f"Speed: {vehicle_state['actual_speed']:>5.2f} m/s | "
                  f"Heading: {vehicle_state['heading']:>6.1f}° | "
                  f"Pos: ({vehicle_state['position_x']:>6.1f}, {vehicle_state['position_y']:>6.1f})")
        
        time.sleep(0.5)

try:
    simulate_vehicle()
except KeyboardInterrupt:
    print("\n\n👋 Vehicle simulator stopped")