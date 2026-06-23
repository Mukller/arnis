# Contributing

## Как помочь

Принимаю PR с:
- Поддержкой новых типов OSM-объектов (мосты, тоннели, железные дороги)
- Улучшением маппинга объектов на блоки Minecraft
- Поддержкой Java Edition (сейчас только Bedrock)
- Оптимизацией больших радиусов (>1км)

## Как делать

```bash
git clone https://github.com/Mukller/arnis
cd arnis
cargo build
```

Проверяйте на нескольких координатах из разных городов.

## Стиль

- `cargo fmt` и `cargo clippy` перед PR
- Новые типы объектов — в `src/mapper.rs`
- Логика работы с API — только в `src/osm.rs`
