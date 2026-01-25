import { WebSocket } from 'ws';
globalThis.WebSocket = WebSocket;

import { Session, Config } from '@eclipse-zenoh/zenoh-ts';

async function main() {
    console.log('Starting WebSocket publisher...');
    
    try {
        console.log('Connecting to ws://localhost:10000...');
        
        // Create config and set locator
        const config = new Config();
        config.locator = "ws://localhost:10000";
        
        // Open session with config
        const session = await Session.open(config);
        
        console.log('✅ Connected!');
        
        let count = 0;
        setInterval(async () => {
            const msg = `Message ${count++}`;
            console.log(`📤 Publishing: ${msg}`);
            await session.put('test/topic', msg);
        }, 1000);
        
        process.on('SIGINT', async () => {
            console.log('\n🛑 Stopping...');
            await session.close();
            process.exit(0);
        });
        
    } catch (error) {
        console.error('❌ Error:', error.message);
    }
}

main();