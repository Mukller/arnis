# arnis

Takes real coordinates from OpenStreetMap and generates a Minecraft world from them.

Roads → dirt paths. Buildings → brick houses. Parks → grass and trees. Water → water. Everything as-is, 1:1 in blocks.

## Usage

```bash
cargo run --release -- --lat 55.7558 --lon 37.6173 --radius 500
```

Options:
```
--lat     center latitude (e.g. 51.5074 for London)
--lon     center longitude
--radius  radius in meters (default 300)
--output  output folder for .mcworld file (default ./world)
--scale   meters per block (default 1)
```

## How it works

1. Queries Overpass API (OSM mirror)
2. Parses GeoJSON: roads, buildings, water, vegetation
3. Projects coordinates to a block grid
4. Generates Minecraft Bedrock Edition structure
5. Saves as `.mcworld` — double-click to open
