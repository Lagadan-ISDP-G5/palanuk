import { WebSocket } from 'ws';
globalThis.WebSocket = WebSocket;

import { Session, Config } from '@eclipse-zenoh/zenoh-ts';

async function main() {
    console.log('Starting WebSocket subscriber...');
    
    try {
        console.log('Connecting to ws://localhost:10000...');
        
        // Create config and set locator
        const config = new Config();
        config.locator = "ws://localhost:10000";
        
        // Open session with config
        const session = await Session.open(config);
        console.log('✅ Connected!');
        
        console.log('Listening for messages...');
        
        // Subscribe to topic
        const subscriber = await session.declareSubscriber('test/topic', (sample) => {
            console.log(`📥 Received: ${sample.payload.toString()}`);
        });
        
        process.on('SIGINT', async () => {
            console.log('\n🛑 Stopping...');
            await subscriber.undeclare();
            await session.close();
            process.exit(0);
        });
        
    } catch (error) {
        console.error('❌ Error:', error.message);
    }
}

main();