import * as zenoh from '@eclipse-zenoh/zenoh-ts';

console.log('All exports from @eclipse-zenoh/zenoh-ts:');
console.log('==========================================');
for (const key of Object.keys(zenoh).sort()) {
    console.log(`- ${key}: ${typeof zenoh[key]}`);
}

// Try to find Config
console.log('\nLooking for Config-related exports:');
for (const key of Object.keys(zenoh)) {
    if (key.toLowerCase().includes('config')) {
        console.log(`Found: ${key} =`, zenoh[key]);
    }
}

// Try to find Session-related exports
console.log('\nLooking for Session-related exports:');
for (const key of Object.keys(zenoh)) {
    if (key.toLowerCase().includes('session')) {
        console.log(`Found: ${key} =`, zenoh[key]);
    }
}