import { writable } from 'svelte/store';

export const telemetryData = writable({
  speed: 0,
  battery: 0,
  position: { x: 0, y: 0 },
  heading: 0
});

export const connectionStatus = writable('disconnected');
export const vehicleState = writable({
  status: 'stopped'
});
export const commandFeedback = writable(null);
export const messageLog = writable([]);

class WebSocketClient {
  constructor(url) {
    this.url = url;
    this.ws = null;
    this.reconnectInterval = 3000;
    this.shouldReconnect = true;
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
          
          if (data.type === 'telemetry') {
            telemetryData.set(data.payload);
          } 
          else if (data.type === 'welcome') {
            console.log('Welcome:', data.message);
          }
          else if (data.type === 'state_update') {
            vehicleState.set(data.payload);
          }
          else if (data.type === 'command_response') {
            // Show command feedback
            commandFeedback.set({
              command: data.command,
              success: data.success,
              message: data.message,
              timestamp: new Date().toLocaleTimeString()
            });
            
            // Clear feedback after 5 seconds
            setTimeout(() => {
              commandFeedback.set(null);
            }, 5000);
          }
          else if (data.key && data.key.includes("itp/state")) {
            telemetryData.set(data.payload || data);
          }
          
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

  send(data) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data));
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

let wsClient = null;

export function initWebSocket(url = 'ws://localhost:8081') {
  if (!wsClient) {
    wsClient = new WebSocketClient(url);
  }
  wsClient.connect();
  return wsClient;
}

export function getWebSocketClient() {
  return wsClient;
}