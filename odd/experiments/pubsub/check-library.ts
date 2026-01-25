// Try to import and see what's available
import * as zenoh from '@eclipse-zenoh/zenoh-ts';

console.log('Zenoh library exports:');
for (const key of Object.keys(zenoh).sort()) {
    console.log(`  ${key}: ${typeof zenoh[key]}`);
}

// Check Session prototype
console.log('\nSession prototype methods:');
if (zenoh.Session) {
    console.log(Object.getOwnPropertyNames(zenoh.Session.prototype));
}

// Check if there's a different way to create session
console.log('\nChecking for factory methods...');
const factoryMethods = Object.keys(zenoh).filter(key => 
    typeof zenoh[key] === 'function' && 
    (key.toLowerCase().includes('create') || key.toLowerCase().includes('open'))
);
console.log('Factory methods:', factoryMethods);