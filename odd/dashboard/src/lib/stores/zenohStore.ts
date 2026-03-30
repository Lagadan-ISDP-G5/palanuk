// src/lib/stores/zenohStore.ts
import { writable, derived } from 'svelte/store';

export const telemetryData = writable({
  speed: 0,
  battery: 0,
  position: { x: 0, y: 0 },
  heading: 0,
  obstacle: false,
  distance: 0,

  // ─── 5V system — raw fields ───
  power_mw: 0,
  load_current_ma: 0,
  bus_voltage_mv: 0,
  shunt_voltage_mv: 0,
  // ─── 5V system — derived fields (pre-calculated by bridge) ───
  power_w: 0,
  current_a: 0,
  voltage_v: 0,

  // ─── Left Motor (lmtr) ───
  lmtr_power_mw: 0,
  lmtr_current_ma: 0,
  lmtr_bus_voltage_mv: 0,
  lmtr_shunt_voltage_mv: 0,
  lmtr_actual_speed: 0,    // encoder speed from palanuk/anc/lmtr-actual-speed

  // ─── Right Motor (rmtr) ───
  rmtr_power_mw: 0,
  rmtr_current_ma: 0,
  rmtr_bus_voltage_mv: 0,
  rmtr_shunt_voltage_mv: 0,
  rmtr_actual_speed: 0,    // encoder speed from palanuk/anc/rmtr-actual-speed

  // ─── Drive state (from control_state) ───
  // 0 = stopped, 1 = forward, 2 = reverse
  drivestate: 0,
});

// ─── Derived store: combined totals across both motors ───
export const motorTotals = derived(telemetryData, ($t) => ({
  total_power_mw:        $t.lmtr_power_mw         + $t.rmtr_power_mw,
  total_current_ma:      $t.lmtr_current_ma        + $t.rmtr_current_ma,
  avg_bus_voltage_mv:    ($t.lmtr_bus_voltage_mv   + $t.rmtr_bus_voltage_mv)   / 2,
  avg_shunt_voltage_mv:  ($t.lmtr_shunt_voltage_mv + $t.rmtr_shunt_voltage_mv) / 2,
}));

export const connectionStatus = writable('disconnected');
export const vehicleState    = writable({ status: 'stopped' });
export const commandFeedback = writable([]);
export const messageLog      = writable([]);


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
        try {
          const data = JSON.parse(event.data);

          if (data.type === 'welcome') {
            console.log('Welcome:', data.message);
          }
          else if (data.type === 'initial_data') {
            this.handleInitialData(data.data);
          }
          else if (data.type === 'component_update') {
            this.handleComponentUpdate(data.component, data.data);
          }
          else if (data.type === 'vehicle_state') {
            vehicleState.set({ status: data.data.status || 'unknown' });
          }
          else if (data.type === 'command_response') {
            commandFeedback.update(log => {
              const entry = {
                command:   data.command,
                success:   data.success,
                message:   data.message,
                timestamp: data.timestamp || new Date().toLocaleTimeString()
              };
              const next = [...log, entry];
              return next.slice(-50); // keep last 50
            });
          }
          else if (data.type === 'control_mode_response') {
            commandFeedback.update(log => {
              const entry = {
                command:   'mode_change',
                success:   data.success,
                message:   data.message,
                timestamp: data.timestamp || new Date().toLocaleTimeString()
              };
              const next = [...log, entry];
              return next.slice(-50);
            });
          }
          // ─── Raw Zenoh topic passthrough ───
          // If the bridge forwards individual Zenoh topics as:
          // { type: "zenoh_topic", key: "palanuk/ec/lmtr/power/mwatts", value: 1234 }
          else if (data.type === 'zenoh_topic') {
            this.handleZenohTopic(data.key, data.value);
          }

          // Only log infrequent messages — skip high-frequency telemetry
          if (data.type !== 'component_update' && data.type !== 'zenoh_topic') {
            messageLog.update(logs => {
              const newLogs = [...logs, {
                time: new Date().toLocaleTimeString(),
                data: data
              }];
              return newLogs.slice(-50);
            });
          }

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

  // ─────────────────────────────────────────────────────────────────────────
  // INITIAL DATA SNAPSHOT (called once on first connect)
  // ─────────────────────────────────────────────────────────────────────────
  private handleInitialData(data: any) {

    // 5V system energy — bridge sends both raw and derived fields
    if (data.energy) {
      telemetryData.update(current => ({
        ...current,
        // Raw fields
        power_mw:         data.energy.power_mw          || 0,
        load_current_ma:  data.energy.load_current_ma    || 0,
        bus_voltage_mv:   data.energy.bus_voltage_mv     || 0,
        shunt_voltage_mv: data.energy.shunt_voltage_mv   || 0,
        // Derived fields (bridge pre-calculates these)
        battery:          data.energy.battery            || 0,
        power_w:          data.energy.power_w            || 0,
        current_a:        data.energy.current_a          || 0,
        voltage_v:        data.energy.voltage_v          || 0,
      }));
    }

    // Left motor initial snapshot
    if (data.lmtr) {
      telemetryData.update(current => ({
        ...current,
        lmtr_power_mw:         data.lmtr.power_mw          || 0,
        lmtr_current_ma:       data.lmtr.current_ma         || 0,
        lmtr_bus_voltage_mv:   data.lmtr.bus_voltage_mv     || 0,
        lmtr_shunt_voltage_mv: data.lmtr.shunt_voltage_mv   || 0,
        lmtr_actual_speed:     data.lmtr.actual_speed       || 0,
      }));
    }

    // Right motor initial snapshot
    if (data.rmtr) {
      telemetryData.update(current => ({
        ...current,
        rmtr_power_mw:         data.rmtr.power_mw          || 0,
        rmtr_current_ma:       data.rmtr.current_ma         || 0,
        rmtr_bus_voltage_mv:   data.rmtr.bus_voltage_mv     || 0,
        rmtr_shunt_voltage_mv: data.rmtr.shunt_voltage_mv   || 0,
        rmtr_actual_speed:     data.rmtr.actual_speed       || 0,
      }));
    }

    // Drive state
    if (data.control_state) {
      const statusMap: Record<number, string> = {
        0: 'stopped',
        1: 'moving_forward',
        2: 'moving_backward'
      };
      vehicleState.set({
        status: statusMap[data.control_state.drivestate] || 'unknown'
      });
    }

    // Navigation
    if (data.navigation) {
      telemetryData.update(current => ({
        ...current,
        obstacle: data.navigation.obstacle || false,
        distance: data.navigation.distance || 0,
      }));
    }
  }

  // ─────────────────────────────────────────────────────────────────────────
  // REAL-TIME COMPONENT UPDATES
  // ─────────────────────────────────────────────────────────────────────────
  private handleComponentUpdate(component: string, data: any) {

    switch (component) {

      // ── 5V system ──
      // The bridge already sends derived fields (power_w, current_a, voltage_v, battery)
      // alongside the raw fields (power_mw, load_current_ma, bus_voltage_mv, shunt_voltage_mv).
      // We store ALL of them — no manual conversion needed here.
      case 'energy':
        telemetryData.update(current => ({
          ...current,
          // Raw fields from Zenoh topics
          power_mw:         data.power_mw          ?? current.power_mw,
          load_current_ma:  data.load_current_ma   ?? current.load_current_ma,
          bus_voltage_mv:   data.bus_voltage_mv     ?? current.bus_voltage_mv,
          shunt_voltage_mv: data.shunt_voltage_mv   ?? current.shunt_voltage_mv,
          // Derived fields pre-calculated by the bridge
          battery:          data.battery            ?? current.battery,
          power_w:          data.power_w            ?? current.power_w,
          current_a:        data.current_a          ?? current.current_a,
          voltage_v:        data.voltage_v          ?? current.voltage_v,
        }));
        break;

      // ── Left Motor ──
      // Bridge should send: { component: "lmtr", data: { power_mw, current_ma, bus_voltage_mv, shunt_voltage_mv } }
      case 'lmtr':
        telemetryData.update(current => ({
          ...current,
          lmtr_power_mw:         data.power_mw          ?? data.power_mwatts         ?? current.lmtr_power_mw,
          lmtr_current_ma:       data.current_ma         ?? data.current_mamps        ?? current.lmtr_current_ma,
          lmtr_bus_voltage_mv:   data.bus_voltage_mv     ?? data.bus_voltage_mvolts   ?? current.lmtr_bus_voltage_mv,
          lmtr_shunt_voltage_mv: data.shunt_voltage_mv   ?? data.shunt_voltage_mvolts ?? current.lmtr_shunt_voltage_mv,
          lmtr_actual_speed:     data.actual_speed       ?? current.lmtr_actual_speed,
        }));
        break;

      case 'rmtr':
        telemetryData.update(current => ({
          ...current,
          rmtr_power_mw:         data.power_mw          ?? data.power_mwatts         ?? current.rmtr_power_mw,
          rmtr_current_ma:       data.current_ma         ?? data.current_mamps        ?? current.rmtr_current_ma,
          rmtr_bus_voltage_mv:   data.bus_voltage_mv     ?? data.bus_voltage_mvolts   ?? current.rmtr_bus_voltage_mv,
          rmtr_shunt_voltage_mv: data.shunt_voltage_mv   ?? data.shunt_voltage_mvolts ?? current.rmtr_shunt_voltage_mv,
          rmtr_actual_speed:     data.actual_speed       ?? current.rmtr_actual_speed,
        }));
        break;

      // ── Drive control state ──
      case 'control_state':
        telemetryData.update(current => ({
          ...current,
          drivestate: data.drivestate ?? current.drivestate,
        }));
        if (data.drivestate !== undefined) {
          const statusMap: Record<number, string> = { 0: 'stopped', 1: 'moving_forward', 2: 'moving_backward' };
          vehicleState.set({ status: statusMap[data.drivestate] || 'unknown' });
        }
        break;

      case 'navigation':
        telemetryData.update(current => ({
          ...current,
          obstacle: data.obstacle ?? current.obstacle,
          distance: data.distance ?? current.distance,
        }));
        if (data.obstacle) console.warn('⚠️ Obstacle detected!');
        break;

      case 'vehicle_state':
        vehicleState.set({ status: data.status || 'unknown' });
        break;

      default:
    }
  }

  // ─────────────────────────────────────────────────────────────────────────
  // ZENOH TOPIC PASSTHROUGH
  // If the ITP bridge forwards individual Zenoh topic messages directly:
  // { type: "zenoh_topic", key: "palanuk/ec/lmtr/power/mwatts", value: 1234 }
  // ─────────────────────────────────────────────────────────────────────────
  private handleZenohTopic(key: string, value: number) {

    const topicMap: Record<string, (v: number) => void> = {
      // ── Left Motor ──
      'palanuk/ec/lmtr/power/mwatts':         (v) => telemetryData.update(c => ({ ...c, lmtr_power_mw: v })),
      'palanuk/ec/lmtr/current/mamps':        (v) => telemetryData.update(c => ({ ...c, lmtr_current_ma: v })),
      'palanuk/ec/lmtr/bus_voltage/mvolts':   (v) => telemetryData.update(c => ({ ...c, lmtr_bus_voltage_mv: v })),
      'palanuk/ec/lmtr/shunt_voltage/mvolts': (v) => telemetryData.update(c => ({ ...c, lmtr_shunt_voltage_mv: v })),

      // ── Right Motor ──
      'palanuk/ec/rmtr/power/mwatts':         (v) => telemetryData.update(c => ({ ...c, rmtr_power_mw: v })),
      'palanuk/ec/rmtr/current/mamps':        (v) => telemetryData.update(c => ({ ...c, rmtr_current_ma: v })),
      'palanuk/ec/rmtr/bus_voltage/mvolts':   (v) => telemetryData.update(c => ({ ...c, rmtr_bus_voltage_mv: v })),
      'palanuk/ec/rmtr/shunt_voltage/mvolts': (v) => telemetryData.update(c => ({ ...c, rmtr_shunt_voltage_mv: v })),
    };

    const handler = topicMap[key];
    if (handler) {
      handler(value);
    }
  }

  send(data: any) {
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

export function sendCommand(command: string) {
  const client = getWebSocketClient();
  if (client) {
    client.send({ type: 'command', payload: command });
  } else {
    console.error('WebSocket not initialized');
  }
}