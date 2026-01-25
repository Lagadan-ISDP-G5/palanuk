import { open, Config } from '@eclipse-zenoh/zenoh-ts';

async function test() {
    console.log('Testing with Config instance...');
    
    try {
        const config = new Config();
        console.log('Config instance created');
        console.log('Config properties:', Object.getOwnPropertyNames(config));
        
        // Try setting properties
        config.locator = 'tcp/localhost:7447';
        console.log('Set locator to:', config.locator);
        
        const session = await open(config);
        console.log('✅ Success with Config instance!');
        
        await session.close();
        
    } catch (error) {
        console.error('❌ Failed:', error.message);
    }
}

test();