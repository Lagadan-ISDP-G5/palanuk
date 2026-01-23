## Zenoh topics

Root topic: `/palanuk`

Namespaces:

- `odd` - Data produced by ODD
- `itp` - Data produced by ITP
- `anc` - Data produced by ANC
- `ec` - Data produced by EC

Data under `ec` (`/palanuk/ec/**`):

- `power/mwatts/<AppFloat>`
- `load_current/mamps/<AppFloat>`
- `shunt_voltage/mvolts/<AppFloat>`
- `bus_voltage/mvolts/<AppFloat>`

For each measurable, child topics may be additionally defined for alternate units.

Data under `/palanuk/odd/**`:

- `loopmode/<AppInteger>` - 0 - Open loop, 1 - Closed loop
- `stop/<AppInteger>` - This is a boolean (1 - true, 0 - false)
- `steer/left/<AppFloat>`
- `steer/right/<AppFloat>`
- `speed/<AppFloat>`
- `drivestate/<AppInteger>` - This is NOT a boolean, but an enum (0 - At Rest, 1 - Forward, 2 - Reverse) 
- `forcepan/<AppInteger>` - 0 - Center, 1 - Reference Left, 2 - Reference Right

Data under `/palanuk/itp/**`:

- `panner/<AppInteger>` - 0 - Center, 1 - Reference Left, 2 - Reference Right

Data under `/palanuk/anc/**`:

- `obstacle/<AppInteger>` - This is a boolean (1 - obstacle detected, 0 - no obstacle detected)
- `distance/<AppFloat>` - Extra feature, not part of 80% integration target
