## Zenoh topics

Root topic: `/palanuk`. Topic value types refer to Rust primitive types. Floats are `f64` to correspond with Python floats, which is 64 bits on almost all platforms.

Namespaces:

- `odd` - Data produced by ODD
- `itp` - Data produced by ITP
- `anc` - Data produced by ANC
- `ec` - Data produced by EC

Data under `ec` (`/palanuk/ec/**`):

- `power/mwatts/<f64>`
- `load_current/mamps/<f64>`
- `shunt_voltage/mvolts/<f64>`
- `bus_voltage/mvolts/<f64>`

For each measurable, child topics may be additionally defined for alternate units.

Data under `/palanuk/odd/**`:

- `loopmode/<u8>` - 0 - Open loop, 1 - Closed loop
- `speed/<f64>`
- `drivestate/<u8>` - This is NOT a boolean, but an enum (0 - At Rest, 1 - Forward, 2 - Reverse) 
- `forcepan/<u8>` - 0 - Center, 1 - Reference Left, 2 - Reference Right

Preliminary implementation on ANC and ODD side for now (9/2/2026):
- `speed/<f64>`
- `stop/<u8>` - This is a boolean (1 - true, 0 - false)
- `loopmode/<u8>` - 0 - Open loop, 1 - Closed loop
- `drivestate/<u8>` - This is NOT a boolean, but an enum (0 - At Rest, 1 - Forward, 2 - Reverse) 
- `forcepan/<u8>` - 0 - Center, 1 - Reference Left, 2 - Reference Right
- `steercmd/<u8>` - 0 - Free, 1 - Hard Left, 2 - Hard Right

Data under `/palanuk/itp/**`:

- `panner/<u8>` - 0 - Center, 1 - Reference Left, 2 - Reference Right

Data under `/palanuk/anc/**`:

- `obstacle/<u8>` - This is a boolean (1 - obstacle detected, 0 - no obstacle detected)
- `distance/<f64>` - Relayed distance sensor reading
