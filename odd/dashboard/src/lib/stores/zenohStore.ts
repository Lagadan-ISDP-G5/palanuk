import { writable } from 'svelte/store';

export interface NavigationState {
    value: number;
    timestamp: string;
}

export interface Coordinates {
    x: number;
    y: number;
    type: string;
    timestamp: string;
}

export const navigationState = writable<NavigationState | null>(null);
export const coordinates = writable<Coordinates | null>(null);
export const isConnected = writable(false);
export const lastUpdate = writable<Date | null>(null);
export const connectionStatus = writable<string>("disconnected");

let ws: WebSocket | null = null;

const stateNames: { [key: number]: string } = {
    0: "IDLE",
    1: "SCANNING",
    2: "PARKING_SPOT_DETECTED",
    3: "APPROACHING",
    4: "ALIGNING",
    5: "PARKING_IN_PROGRESS",
    6: "PARKING_COMPLETE",
    7: "OBSTACLE_DETECTED",
    8: "ERROR"
};

export function connectToZenoh(url: string = "ws://localhost:8080/ws") {
    if (ws) {
        return; // Already connected
    }

    connectionStatus.set("connecting");

    try {
        ws = new WebSocket(url);

        ws.onopen = () => {
            console.log("Connected to Zenoh bridge");
            isConnected.set(true);
            connectionStatus.set("connected");
        };

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                console.log("Received:", data);

                // Route data based on key
                if (data.key.includes("itp/state")) {
                    const stateValue = parseInt(data.value);
                    navigationState.set({
                        value: stateValue,
                        timestamp: data.timestamp
                    });
                } else if (data.key.includes("coordinates")) {
                    const coords = JSON.parse(data.value);
                    coordinates.set({
                        x: coords.x,
                        y: coords.y,
                        type: coords.type || "unknown",
                        timestamp: data.timestamp
                    });
                }

                lastUpdate.set(new Date());
            } catch (error) {
                console.error("Error processing message:", error);
            }
        };

        ws.onerror = (error) => {
            console.error("WebSocket error:", error);
            isConnected.set(false);
            connectionStatus.set("error");
        };

        ws.onclose = () => {
            console.log("Disconnected from Zenoh bridge");
            isConnected.set(false);
            connectionStatus.set("disconnected");
            ws = null;
            // Attempt reconnection after 3 seconds
            setTimeout(() => connectToZenoh(url), 3000);
        };
    } catch (error) {
        console.error("Failed to create WebSocket:", error);
        connectionStatus.set("error");
        isConnected.set(false);
    }
}

export function disconnectFromZenoh() {
    if (ws) {
        ws.close();
        ws = null;
    }
    isConnected.set(false);
    connectionStatus.set("disconnected");
}

export function getStateNameByValue(value: number): string {
    return stateNames[value] || "UNKNOWN";
}
