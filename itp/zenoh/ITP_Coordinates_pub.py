import time
import json
from typing import Tuple, Optional

import zenoh


class CoordinatesPublisher:
    """Publisher for x,y coordinates (parking slot or lane tracking)"""
    
    COORDS_KEY = "parking_robot/coordinates"
    
    def __init__(self, session: zenoh.Session):
        self.session = session
        self.pub = session.declare_publisher(self.COORDS_KEY)
    
    def publish_coordinates(self, x: float, y: float, coord_type: str = "parking_slot"):
        """
        Publish coordinates
        coord_type: "parking_slot", "lane_tracking", etc.
        """
        payload = json.dumps({
            "x": x,
            "y": y,
            "type": coord_type,
            "timestamp": time.time()
        })
        self.pub.put(payload)
        print(f"Published {coord_type} coordinates: ({x}, {y})")


def main():
    """Example: Publish coordinates"""
    conf = zenoh.Config()
    
    with zenoh.open(conf) as session:
        coord_pub = CoordinatesPublisher(session)
        
        try:
            # Simulate receiving coordinates and publishing them
            test_coordinates = [
                (10.5, 20.3, "parking_slot"),
                (11.2, 21.5, "parking_slot"),
                (12.0, 22.1, "parking_slot"),
            ]
            
            for x, y, coord_type in test_coordinates:
                time.sleep(1)
                coord_pub.publish_coordinates(x, y, coord_type)
        
        except KeyboardInterrupt:
            print("Publisher stopped")


if __name__ == "__main__":
    main()