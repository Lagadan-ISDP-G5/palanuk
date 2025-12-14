# Zenoh deployment

## How to deploy Zenoh

The runtime implements two Zenoh tasks: `cu-zenoh-sink` and `cu-zenoh-src`.

When these tasks are instantiated in the RON, their `config` field should **at least** include the following fields:

- `topic`

The topic should **at least** be an empty string (`""`). By default the task would create a `/palanuk` topic. Which means that if multiple topics are instantiated with the `topic` field set to an empty string, it might result in a compile/runtime error.

## Proper Copper deployment (IMPORTANT)

However, a complete deployment should have the `config` field of a `cu-zenoh-sink` or `cu-zenoh-src` task be populated with the following:

- `topic` - value: `/palanuk/your_key_here`
- `zenoh_config_file` - value: `/local/relative/path/to/zenoh/config/file/on/the/UGV`

## Important docs

Set up REST API plugin for a router:

[https://github.com/eclipse-zenoh/zenoh-ts/blob/main/zenoh-plugin-remote-api/README.md]()

## Topology

- 1 Router node: Resides on the base station laptop
- 3 Peer nodes: x2 resides on the base station laptop, x1 resides on the UGV

## Peer nodes

There are 3 peer nodes:

- Peer 1: UGV pubsub
- Peer 2: ODD TypeScript pubsub over REST API [https://github.com/eclipse-zenoh/zenoh-ts/blob/main/zenoh-plugin-remote-api/README.md]()
- Peer 3: ITP Python pubsub [https://github.com/eclipse-zenoh/zenoh-python]()

## Code examples

### ODD code example for pubsub

[https://github.com/eclipse-zenoh/zenoh-ts/blob/main/zenoh-ts/examples/browser/chat/src/chat_session.ts]()
