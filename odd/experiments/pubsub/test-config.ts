import { Config } from '@eclipse-zenoh/zenoh-ts';

console.log('Config prototype methods:');
console.log(Object.getOwnPropertyNames(Config.prototype));

const config = new Config();
console.log('\nConfig instance properties:');
console.log(Object.getOwnPropertyNames(config));