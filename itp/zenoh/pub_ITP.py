import asyncio
import json
import time
import logging

import zenoh


logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class NavigationStatePublisher:
    """Publisher for navigation states from ITP"""
    
    STATE_KEY = "parking_robot/itp/state"
    
    def __init__(self, session: zenoh.Session):
        self.session = session
        self.pub = session.declare_publisher(self.STATE_KEY)
    
    def publish_state(self, state: int):
        """Publish navigation state"""
        payload = json.dumps({
            "state": state,
            "timestamp": time.time()
        })
        self.pub.put(str(state))
        logger.info(f"Published state: {state}")


def main():
    """Main publisher function"""
    zenoh.init_log_from_env_or("error")
    
    conf = zenoh.Config()
    
    logger.info("Opening session for ITP Publisher...")
    with zenoh.open(conf) as session:
        publisher = NavigationStatePublisher(session)
        
        # Simulate publishing different states
        states = [0, 1, 2, 3, 4, 5, 6]
        
        try:
            while True:
                for state in states:
                    time.sleep(2)
                    publisher.publish_state(state)
        except KeyboardInterrupt:
            logger.info("Publisher stopped")


if __name__ == "__main__":
    main()