import { Session, Config } from '@eclipse-zenoh/zenoh-ts';

async function test() {
    console.log('Testing different config formats...\n');
    
    const testConfigs = [
        { name: 'Empty object', config: {} },
        { name: 'With mode only', config: { mode: "client" } },
        { name: 'With endpoints only', config: { connect: { endpoints: ["tcp/localhost:7447"] } } },
        { name: 'Full config', config: { 
            mode: "client",
            connect: { endpoints: ["tcp/localhost:7447"] }
        } },
        { name: 'Peer mode', config: { mode: "peer" } },
        { name: 'Just mode client', config: { mode: "client" } }
    ];
    
    for (const test of testConfigs) {
        console.log(`Testing: ${test.name}...`);
        console.log('Config:', JSON.stringify(test.config, null, 2));
        
        try {
            const session = await Session.open(test.config);
            console.log(`✅ ${test.name} SUCCESS!`);
            await session.close();
        } catch (error) {
            console.log(`❌ ${test.name} FAILED: ${error.message}`);
        }
        console.log('---\n');
    }
}

test().catch(console.error);