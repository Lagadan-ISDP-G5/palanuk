// src/lib/zenohStore.ts
// src/lib/stores/zenohStore.ts
import { writable } from 'svelte/store';

export const telemetryData = writable({
  speed: 0,
  battery: 0,
  position: { x: 0, y: 0 },
  heading: 0,
  obstacle: false,
  distance: 0,
  power_w: 0,
  current_a: 0,
  voltage_v: 0
});

export const connectionStatus = writable('disconnected');
export const vehicleState = writable({ status: 'stopped' });
export const commandFeedback = writable(null);
export const messageLog = writable([]);


class WebSocketClient {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnectInterval = 3000;
  private shouldReconnect = true;

  constructor(url: string) {
    this.url = url;
  }

  connect() {
    try {
      this.ws = new WebSocket(this.url);
      
      this.ws.onopen = () => {
        console.log('WebSocket connected');
        connectionStatus.set('connected');
      };

      this.ws.onmessage = (event) => {
        console.log('Received:', event.data);
        
        try {
          const data = JSON.parse(event.data);
          
          // Handle different message types
          if (data.type === 'welcome') {
            console.log('Welcome:', data.message);
          }
          else if (data.type === 'initial_data') {
            console.log('Received initial dashboard data');
            this.handleInitialData(data.data);
          }
          else if (data.type === 'component_update') {
            this.handleComponentUpdate(data.component, data.data);
          }
          else if (data.type === 'vehicle_state') {
            vehicleState.set({
              status: data.data.status || 'unknown'
            });
          }
          else if (data.type === 'command_response') {
            commandFeedback.set({
              command: data.command,
              success: data.success,
              message: data.message,
              timestamp: data.timestamp || new Date().toLocaleTimeString()
            });
            setTimeout(() => commandFeedback.set(null), 5000);
          }
          else if (data.type === 'control_mode_response') {
            commandFeedback.set({
              command: 'mode_change',
              success: data.success,
              message: data.message,
              timestamp: data.timestamp || new Date().toLocaleTimeString()
            });
            setTimeout(() => commandFeedback.set(null), 3000);
          }

          // Update message log for all messages
          messageLog.update(logs => {
            const newLogs = [...logs, {
              time: new Date().toLocaleTimeString(),
              data: data
            }];
            return newLogs.slice(-50);
          });
          
        } catch (error) {
          console.error('Error processing message:', error);
        }
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        connectionStatus.set('error');
      };

      this.ws.onclose = () => {
        console.log('WebSocket disconnected');
        connectionStatus.set('disconnected');
        
        if (this.shouldReconnect) {
          setTimeout(() => this.connect(), this.reconnectInterval);
        }
      };

    } catch (error) {
      console.error('Connection error:', error);
      connectionStatus.set('error');
    }
  }

private handleInitialData(data: any) {  // parameter is 'data'
  console.log('Processing initial dashboard data');
  
  // Update telemetry from initial data
  if (data.energy) {  // ✅ Use 'data', not 'initialData'
    telemetryData.update(current => ({
      ...current,
      battery: data.energy.battery || 0,  // ✅
      power_w: data.energy.power_w || 0,
      current_a: data.energy.load_current_a || 0,
      voltage_v: data.energy.bus_voltage_v || 0
    }));
  }

  if (data.control_state) {  // ✅
    const statusMap = {
      0: 'stopped',
      1: 'moving_forward',
      2: 'moving_backward'
    };
    vehicleState.set({
      status: statusMap[data.control_state.drivestate] || 'unknown'  // ✅
    });
  }
  
  if (data.navigation) {  // ✅
    telemetryData.update(current => ({
      ...current,
      obstacle: data.navigation.obstacle || false,  // ✅
      distance: data.navigation.distance || 0
    }));
  }
}

    private handleComponentUpdate(component: string, data: any) {
    console.log(`Component update: ${component}`, data);
    
    switch(component) {
      case 'navigation':
        telemetryData.update(current => ({
          ...current,
          obstacle: data.obstacle || current.obstacle,
          distance: data.distance || current.distance
        }));
        
        // Show obstacle warning
        if (data.obstacle) {
          console.warn('⚠️ Obstacle detected!');
        }
        break;
        
      case 'energy':
        telemetryData.update(current => ({
          ...current,
          battery: data.battery ?? current.battery,
          power_w: data.power_w ?? current.power_w,
          current_a: data.load_current_ma ? (data.load_current_ma / 1000) : current.current_a,
          voltage_v: data.bus_voltage_mv ? (data.bus_voltage_mv / 1000) : current.voltage_v
        }));
        break;
        
      case 'vehicle_state':
        vehicleState.set({
          status: data.status || 'unknown'
        });
        break;
        
      default:
        console.log(`Unhandled component: ${component}`);
    }
  }

  send(data: any) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data));
      console.log('Sent:', data);
    } else {
      console.warn('WebSocket is not connected');
    }
  }

  disconnect() {
    this.shouldReconnect = false;
    if (this.ws) {
      this.ws.close();
    }
  }
}

let wsClient: WebSocketClient | null = null;

export function initWebSocket(url: string = 'ws://localhost:8081') {
  if (!wsClient) {
    wsClient = new WebSocketClient(url);
  }
  wsClient.connect();
  return wsClient;
}

export function getWebSocketClient() {
  return wsClient;
}

// Helper function to send commands
export function sendCommand(command: string) {
  const client = getWebSocketClient();
  if (client) {
    client.send({
      type: 'command',
      payload: command
    });
  } else {
    console.error('WebSocket not initialized');
  }
}

