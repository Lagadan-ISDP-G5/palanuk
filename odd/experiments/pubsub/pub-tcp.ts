import { Session } from '@eclipse-zenoh/zenoh-ts';

async function main() {
    console.log('Starting TCP publisher...');
    
    try {
        console.log('Connecting to TCP localhost:7447...');
        
        // Use the correct format for TCP
        const session = await Session.open({
            connect: {
                endpoints: ["tcp/localhost:7447"]
            }
        });
        
        console.log('✅ Connected via TCP!');
        
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