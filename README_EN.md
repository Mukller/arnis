<div align="center">

[Русский](README.md) • **English**

</div>

# arnis

Takes real coordinates from OpenStreetMap and generates a Minecraft world from them.

Roads → dirt paths. Buildings → brick houses. Parks → grass and trees.
Water → water. Everything as-is, 1:1 scale in blocks.

## Run

```bash
cargo run --release -- --lat 55.7558 --lon 37.6173 --radius 500
```

Flags:
```
--lat     center latitude
--lon     center longitude
--radius  radius in meters (default 300)
--output  output folder for .mcworld file (default ./world)
--scale   meters per block (default 1)
```

## How it works

1. Fetches data from Overpass API (OSM mirror)
2. Parses GeoJSON: roads, buildings, water, vegetation
3. Projects coordinates into a block grid
4. Generates Minecraft Bedrock Edition structure
5. Saves as `.mcworld` — double-click to open

## Examples

```bash
# Red Square, Moscow
cargo run --release -- --lat 55.7539 --lon 37.6208 --radius 400

# Central Berlin
cargo run --release -- --lat 52.5200 --lon 13.4050 --radius 600
```

## Requirements

- Rust 1.75+
- Internet access (Overpass API)
