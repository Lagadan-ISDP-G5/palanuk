import asyncio
import json
import logging
from typing import Set, List

import zenoh
from aiohttp import web


class ZenohWebSocketBridge:
    def __init__(self, zenoh_keys: List[str]):
        self.zenoh_keys = zenoh_keys
        self.session = None
        self.subs = []
        self.clients: Set[web.WebSocketResponse] = set()
        self.loop = None
        logging.basicConfig(level=logging.INFO)
        self.logger = logging.getLogger(__name__)

    async def init_zenoh(self):
        """Initialize Zenoh session and subscribers"""
        self.loop = asyncio.get_running_loop()
        conf = zenoh.Config()
        self.session = zenoh.open(conf)
        
        def zenoh_callback(sample: zenoh.Sample):
            """Callback when Zenoh receives data"""
            if self.loop and not self.loop.is_closed():
                asyncio.run_coroutine_threadsafe(
                    self.broadcast_to_clients(sample),
                    self.loop
                )
        
        # Subscribe to all keys
        for key in self.zenoh_keys:
            sub = self.session.declare_subscriber(key, zenoh_callback)
            self.subs.append(sub)
            self.logger.info(f"Subscribed to Zenoh key: {key}")

    async def broadcast_to_clients(self, sample: zenoh.Sample):
        """Broadcast Zenoh data to all connected WebSocket clients"""
        try:
            payload = sample.payload.to_string()
            message = json.dumps({
                "key": str(sample.key_expr),
                "value": payload,
                "timestamp": str(sample.timestamp)
            })
            
            disconnected_clients = set()
            for client in self.clients:
                try:
                    await client.send_str(message)
                except Exception as e:
                    self.logger.warning(f"Client send failed: {e}")
                    disconnected_clients.add(client)
            
            self.clients -= disconnected_clients
        except Exception as e:
            self.logger.error(f"Broadcast error: {e}")

    async def websocket_handler(self, request):
        """Handle WebSocket connections"""
        ws = web.WebSocketResponse()
        await ws.prepare(request)
        self.clients.add(ws)
        self.logger.info(f"Client connected. Total clients: {len(self.clients)}")
        
        try:
            async for msg in ws:
                if msg.type == web.WSMsgType.TEXT:
                    self.logger.info(f"Received from client: {msg.data}")
                elif msg.type == web.WSMsgType.ERROR:
                    self.logger.error(f"WebSocket error: {ws.exception()}")
        finally:
            self.clients.discard(ws)
            self.logger.info(f"Client disconnected. Total clients: {len(self.clients)}")
        
        return ws

    async def cleanup(self):
        """Cleanup Zenoh resources"""
        for sub in self.subs:
            if sub:
                sub.undeclare()
        if self.session:
            self.session.close()


async def create_app(zenoh_keys: List[str] = None):
    """Create aiohttp application"""
    if zenoh_keys is None:
        zenoh_keys = [
            "parking_robot/itp/state",
            "parking_robot/coordinates"
        ]
    
    bridge = ZenohWebSocketBridge(zenoh_keys)
    await bridge.init_zenoh()
    
    app = web.Application()
    app.router.add_get("/ws", bridge.websocket_handler)
    
    async def on_shutdown(app):
        await bridge.cleanup()
    
    app.on_shutdown.append(on_shutdown)
    return app, bridge


async def main():
    """Main async entry point"""
    app, bridge = await create_app()
    
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, "0.0.0.0", 8081)
    await site.start()
    
    logger = logging.getLogger(__name__)
    logger.info("WebSocket server started on ws://0.0.0.0:8081/ws")
    
    try:
        await asyncio.sleep(3600 * 24)  # Run for 24 hours
    except KeyboardInterrupt:
        logger.info("Shutting down...")
    finally:
        await bridge.cleanup()
        await runner.cleanup()


if __name__ == "__main__":
    asyncio.run(main())