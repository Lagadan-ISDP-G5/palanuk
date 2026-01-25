import { open } from '@eclipse-zenoh/zenoh-ts';

async function test() {
    console.log('Testing open() function with config...');
    
    const tests = [
        { config: {}, name: 'Empty object' },
        { config: { mode: 'peer' }, name: 'Peer mode' },
        { config: { mode: 'client' }, name: 'Client mode' },
        { config: { locator: 'tcp/localhost:7447' }, name: 'With locator' },
        { config: { connect: { endpoints: ['tcp/localhost:7447'] } }, name: 'With endpoints' }
    ];
    
    for (const testCase of tests) {
        console.log(`\nTrying: ${testCase.name}...`);
        console.log('Config:', JSON.stringify(testCase.config));
        
        try {
            const session = await open(testCase.config);
            console.log(`✅ ${testCase.name} succeeded!`);
            
            // Quick test
            await session.put('test', 'hello');
            console.log('  Published test message');
            
            await session.close();
            console.log('  Session closed');
            
        } catch (error) {
            console.log(`❌ ${testCase.name} failed: ${error.message}`);
        }
    }
}

test();