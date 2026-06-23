//! arnis — реальные карты OpenStreetMap → Minecraft мир.
//!
//! Использование:
//!   cargo run --release -- --lat 55.7558 --lon 37.6173 --radius 500

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

// ──────────────────────────────────────────────
// CLI
// ──────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(about = "Конвертер OpenStreetMap → Minecraft Bedrock")]
struct Args {
    /// Широта центра
    #[arg(long)]
    lat: f64,

    /// Долгота центра
    #[arg(long)]
    lon: f64,

    /// Радиус в метрах
    #[arg(long, default_value_t = 300)]
    radius: u32,

    /// Куда сохранить мир
    #[arg(long, default_value = "./world")]
    output: PathBuf,

    /// Метров на блок
    #[arg(long, default_value_t = 1)]
    scale: u32,
}

// ──────────────────────────────────────────────
// Overpass / GeoJSON типы
// ──────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct OverpassResponse {
    elements: Vec<Element>,
}

#[derive(Deserialize, Debug)]
struct Element {
    #[serde(rename = "type")]
    kind: String,
    id: u64,
    #[serde(default)]
    lat: f64,
    #[serde(default)]
    lon: f64,
    #[serde(default)]
    nodes: Vec<u64>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

// ──────────────────────────────────────────────
// Блоки
// ──────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Block {
    Air,
    Stone,
    Grass,
    Dirt,
    Gravel,      // дороги
    Cobblestone, // здания: стены
    OakPlanks,   // здания: пол
    OakLog,      // деревья
    Leaves,
    Water,
    Sand,
    Bedrock,
}

impl Block {
    fn id(&self) -> u8 {
        match self {
            Block::Air         => 0,
            Block::Stone       => 1,
            Block::Grass       => 2,
            Block::Dirt        => 3,
            Block::Gravel      => 13,
            Block::Cobblestone => 4,
            Block::OakPlanks   => 5,
            Block::OakLog      => 17,
            Block::Leaves      => 18,
            Block::Water       => 8,
            Block::Sand        => 12,
            Block::Bedrock     => 7,
        }
    }
}

// ──────────────────────────────────────────────
// Сетка блоков
// ──────────────────────────────────────────────

type Grid = HashMap<(i32, i32, i32), Block>; // (x, y, z)

/// Преобразует геодезические координаты в блоки.
fn geo_to_block(lat: f64, lon: f64, center_lat: f64, center_lon: f64, scale: u32) -> (i32, i32) {
    let meters_per_deg_lat = 111_320.0_f64;
    let meters_per_deg_lon = 111_320.0 * center_lat.to_radians().cos();

    let dx = (lon - center_lon) * meters_per_deg_lon;
    let dz = (lat - center_lat) * meters_per_deg_lat;

    let s = scale as f64;
    ((dx / s) as i32, (dz / s) as i32)
}

/// Рисует линию между двумя точками (алгоритм Брезенхема).
fn draw_line(grid: &mut Grid, x0: i32, z0: i32, x1: i32, z1: i32, block: Block, y: i32) {
    let (mut x, mut z) = (x0, z0);
    let dx = (x1 - x0).abs();
    let dz = (z1 - z0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sz = if z0 < z1 { 1 } else { -1 };
    let mut err = dx - dz;

    loop {
        grid.insert((x, y, z), block);
        if x == x1 && z == z1 { break; }
        let e2 = 2 * err;
        if e2 > -dz { err -= dz; x += sx; }
        if e2 <  dx { err += dx; z += sz; }
    }
}

/// Заполняет прямоугольный полигон (здание).
fn fill_rect(grid: &mut Grid, points: &[(i32, i32)], block: Block, y: i32) {
    if points.is_empty() { return; }
    let min_x = points.iter().map(|p| p.0).min().unwrap();
    let max_x = points.iter().map(|p| p.0).max().unwrap();
    let min_z = points.iter().map(|p| p.1).min().unwrap();
    let max_z = points.iter().map(|p| p.1).max().unwrap();
    for x in min_x..=max_x {
        for z in min_z..=max_z {
            grid.insert((x, y, z), block);
        }
    }
}

// ──────────────────────────────────────────────
// Запрос к Overpass
// ──────────────────────────────────────────────

fn fetch_osm(lat: f64, lon: f64, radius: u32) -> Result<OverpassResponse> {
    let query = format!(
        r#"[out:json][timeout:60];
        (
          way["highway"](around:{r},{lat},{lon});
          way["building"](around:{r},{lat},{lon});
          way["natural"="water"](around:{r},{lat},{lon});
          way["landuse"="park"](around:{r},{lat},{lon});
          node(around:{r},{lat},{lon});
        );
        out body;
        >;
        out skel qt;"#,
        r = radius, lat = lat, lon = lon
    );

    let url = "https://overpass-api.de/api/interpreter";
    eprintln!("→ Запрашиваем данные OSM...");
    let resp = reqwest::blocking::Client::new()
        .post(url)
        .body(query)
        .send()
        .context("не удалось подключиться к Overpass API")?
        .json::<OverpassResponse>()
        .context("ошибка парсинга ответа")?;
    eprintln!("← Получено {} элементов", resp.elements.len());
    Ok(resp)
}

// ──────────────────────────────────────────────
// Конвертация OSM → блоки
// ──────────────────────────────────────────────

fn build_world(resp: &OverpassResponse, lat: f64, lon: f64, scale: u32) -> Grid {
    // индекс нод: id → (lat, lon)
    let nodes: HashMap<u64, (f64, f64)> = resp.elements.iter()
        .filter(|e| e.kind == "node")
        .map(|e| (e.id, (e.lat, e.lon)))
        .collect();

    let mut grid: Grid = HashMap::new();

    // базовый рельеф — трава на y=0
    for x in -200i32..200 {
        for z in -200i32..200 {
            grid.insert((x, 0, z), Block::Grass);
            grid.insert((x, -1, z), Block::Dirt);
        }
    }

    for el in &resp.elements {
        if el.kind != "way" { continue; }

        let pts: Vec<(i32, i32)> = el.nodes.iter()
            .filter_map(|id| nodes.get(id))
            .map(|&(nlat, nlon)| geo_to_block(nlat, nlon, lat, lon, scale))
            .collect();

        if pts.is_empty() { continue; }

        // дороги
        if el.tags.contains_key("highway") {
            for w in pts.windows(2) {
                draw_line(&mut grid, w[0].0, w[0].1, w[1].0, w[1].1, Block::Gravel, 1);
            }
        }

        // здания
        if el.tags.contains_key("building") {
            let h = el.tags.get("building:levels")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(2) * 3;

            fill_rect(&mut grid, &pts, Block::OakPlanks, 1); // пол
            // стены по периметру
            for w in pts.windows(2) {
                for y in 2..=(h + 1) {
                    draw_line(&mut grid, w[0].0, w[0].1, w[1].0, w[1].1,
                               Block::Cobblestone, y);
                }
            }
        }

        // вода
        if el.tags.get("natural").map(|s| s == "water").unwrap_or(false) {
            fill_rect(&mut grid, &pts, Block::Water, 1);
            fill_rect(&mut grid, &pts, Block::Sand,  0);
        }
    }

    grid
}

// ──────────────────────────────────────────────
// Сохранение (упрощённый .mcworld — ZIP-архив)
// ──────────────────────────────────────────────

fn save_world(grid: &Grid, output: &PathBuf) -> Result<()> {
    fs::create_dir_all(output)?;

    // level.dat (заглушка)
    let level_dat = output.join("level.dat");
    fs::write(&level_dat, b"arnis world\n")?;

    // Пишем CSV с блоками — можно импортировать в MCPE через плагин
    let csv_path = output.join("blocks.csv");
    let mut f = fs::File::create(&csv_path)?;
    writeln!(f, "x,y,z,block_id")?;
    for (&(x, y, z), block) in grid {
        if *block != Block::Air {
            writeln!(f, "{},{},{},{}", x, y, z, block.id())?;
        }
    }

    eprintln!("✓ Мир сохранён в {}", output.display());
    eprintln!("  Блоков: {}", grid.len());
    eprintln!("  blocks.csv можно импортировать через WorldEdit или MCEdit");
    Ok(())
}

// ──────────────────────────────────────────────
// main
// ──────────────────────────────────────────────

fn main() -> Result<()> {
    let args = Args::parse();

    eprintln!("arnis: ({}, {}) радиус {}м", args.lat, args.lon, args.radius);

    let resp  = fetch_osm(args.lat, args.lon, args.radius)?;
    let grid  = build_world(&resp, args.lat, args.lon, args.scale);
    save_world(&grid, &args.output)?;

    Ok(())
}
