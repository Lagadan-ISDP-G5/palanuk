
# Subscriber for Algorithm & Control and Display Dashboard
# Receives integer navigation states from ITP (Image Transmission & Processing)
#

import time

import zenoh


# Navigation state definitions (must match publisher)
class NavigationState:
    IDLE = 0
    SCANNING = 1
    PARKING_SPOT_DETECTED = 2
    APPROACHING = 3
    ALIGNING = 4
    PARKING_IN_PROGRESS = 5
    PARKING_COMPLETE = 6
    OBSTACLE_DETECTED = 7
    ERROR = 8

    @staticmethod
    def to_string(state: int) -> str:
        """Convert integer state to readable string"""
        states_map = {
            0: "IDLE",
            1: "SCANNING",
            2: "PARKING_SPOT_DETECTED",
            3: "APPROACHING",
            4: "ALIGNING",
            5: "PARKING_IN_PROGRESS",
            6: "PARKING_COMPLETE",
            7: "OBSTACLE_DETECTED",
            8: "ERROR",
        }
        return states_map.get(state, f"UNKNOWN({state})")


class StateListener:
    """Listener to handle incoming navigation states"""

    def __init__(self, name: str = "Subscriber"):
        self.name = name
        self.last_state = None

    def on_state_received(self, sample: zenoh.Sample):
        """Callback function when state is received"""
        try:
            # Parse the integer state from payload
            state = int(sample.payload.to_string())
            
            if state != self.last_state:
                state_name = NavigationState.to_string(state)
                print(f"[{self.name}] New state received: {state} ({state_name})")
                self.last_state = state
            
            # Handle state-specific actions
            self.handle_state(state)
            
        except ValueError:
            print(f"[{self.name}] Error: Could not parse state as integer: {sample.payload.to_string()}")

    def handle_state(self, state: int):
        """Override this method to handle specific states"""
        pass


class AlgorithmControlListener(StateListener):
    """Listener for Algorithm & Control component"""

    def __init__(self):
        super().__init__("Algorithm & Control")

    def handle_state(self, state: int):
        """Handle states specific to Algorithm & Control"""
        if state == NavigationState.PARKING_SPOT_DETECTED:
            print(f"  → AC: Initiating parking algorithm...")
        elif state == NavigationState.ALIGNING:
            print(f"  → AC: Computing alignment vectors...")
        elif state == NavigationState.PARKING_COMPLETE:
            print(f"  → AC: Parking complete, storing position...")


class DisplayDashboardListener(StateListener):
    """Listener for Display Dashboard component"""

    def __init__(self):
        super().__init__("Display Dashboard")

    def handle_state(self, state: int):
        """Handle states specific to Display Dashboard"""
        if state == NavigationState.PARKING_SPOT_DETECTED:
            print(f"  → Dashboard: Displaying parking spot on map...")
        elif state == NavigationState.OBSTACLE_DETECTED:
            print(f"  → Dashboard: WARNING - Obstacle detected!")
        elif state == NavigationState.ERROR:
            print(f"  → Dashboard: ERROR state - displaying alert to user...")


def main(conf: zenoh.Config, key: str):
    """
    Main subscriber function
    Receives integer navigation states from ITP
    """
    zenoh.init_log_from_env_or("error")

    print("Opening session for State Subscriber...")
    with zenoh.open(conf) as session:
        print(f"Declaring Subscriber on '{key}'...")

        # Create listeners for different components
        ac_listener = AlgorithmControlListener()
        dashboard_listener = DisplayDashboardListener()

        def listener(sample: zenoh.Sample):
            """Unified listener that forwards to component listeners"""
            ac_listener.on_state_received(sample)
            dashboard_listener.on_state_received(sample)

        # Subscribe to the key
        session.declare_subscriber(key, listener)

        print("Waiting for navigation states...")
        print("Press CTRL-C to quit...\n")

        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            print("\nSubscriber stopped by user")


if __name__ == "__main__":
    import argparse

    import common

    parser = argparse.ArgumentParser(
        prog="sub_states",
        description="State Subscriber - receives integer navigation states from ITP"
    )
    common.add_config_arguments(parser)
    parser.add_argument(
        "--key",
        "-k",
        dest="key",
        default="parking_robot/itp/state",
        type=str,
        help="The key expression to subscribe to for navigation states.",
    )

    args = parser.parse_args()
    conf = common.get_config_from_args(args)

    main(conf, args.key)
