import { Session } from '@eclipse-zenoh/zenoh-ts';

async function main() {
    console.log('Starting TCP subscriber...');
    
    try {
        console.log('Connecting to TCP localhost:7447...');
        
        // Use the correct format for TCP
        const session = await Session.open({
            connect: {
                endpoints: ["tcp/localhost:7447"]
            }
        });
        
        console.log('✅ Connected via TCP!');
        
        console.log('Listening for messages...');
        
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