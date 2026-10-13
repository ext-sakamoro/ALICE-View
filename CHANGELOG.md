# Changelog

All notable changes to ALICE-View are documented here.

## [Unreleased]

### Changed
- 依存の要求を公開版に追従した: `alice-sdf` `5.0` → `6`、`alice-analytics` `0.3` → `0.5`、`alice-physics` `2.0` → `3.0`、`alice-db` `0.3.0-beta.2` → `0.4` (どの crate も先に進んでいて、旧い要求では `cargo metadata` が解決できなかった) 使っている API に変更は無く、code の変更は無い 実測: `cargo build --lib`、`cargo test --all-features` (lib 148 / bin 122 / 統合 3 件) と `cargo check --lib --features analytics,physics,db` が exit 0、`cargo clippy --all-targets --all-features -- -D warnings` が警告 0
- `.github/actions/alice-stubs` が stub の版と feature を literal で書くのをやめ、`.github/make-stubs.py --allow-stub-all` に委譲するようにした これで版の出所は `Cargo.toml` 1 箇所になる (従来は `ci.yml` が導出、本 action が literal で、要求を動かすたびに 2 箇所の同期が要った) `--allow-stub-all` は「消費側の lib を既定 feature で compile しない job」(cargo audit / deny / semver-checks / fuzz) 専用で、flag を付けなければ従来どおり compile される dep への stub を拒否する
- `make-stubs.py` が `[features]` 表の `<dep>/<feat>` 記法 (`db = ["dep:alice-db", "alice-db/fs"]`) を stub の feature として拾うようにした 拾わないと `depends on alice-db with feature fs but alice-db does not have that feature` で resolve 段から落ちる
- `alice-sdf` の要求を `4.0` から `5.0` に上げた (5.0 で変わった API の利用は無く、code の変更は無い)

### Fixed
- Security & Hygiene の `coverage` と `semver-checks` は何も検査せずに success になっていた `coverage` は lib が compile する sibling を空の stub にして `cargo llvm-cov --lib` を走らせ E0432 で compile に失敗し、`semver-checks` は最新の cargo-semver-checks が rustc 1.93 以上を要求するのに `rust-toolchain.toml` は 1.92.0 で、どちらも job の `continue-on-error: true` (と `|| true`) が失敗を隠していた 両 job とも `ALICE-SDF` / `ALICE-LOL` / `ALICE-Zip` を実 checkout し (`semver-checks` は任意 feature も rustdoc にかけるので `ALICE-Analytics` / `ALICE-Physics` / `ALICE-DB` も)、`continue-on-error` を外した `semver-checks` は cargo-semver-checks を 1.92 の rustdoc を読める 0.49.0 に固定し (`rust-toolchain.toml` を 1.93 以上にしたら外す)、HEAD~1 を隣の checkout として比較し (`--baseline-rev` では path 依存が解決しない)、比較が 0 件・tool の失敗・semver の破壊 (版を上げていないもの) をすべて fail にした alice-view は他の repository (ALICE-Eco-System / ALICE-View-Studio / ALICE-Animation) が依存しているので破壊は止める 比較した版と件数、coverage の行カバレッジ (基準 44.94%、閾値はまだ無い) は job の summary に出す workflow 全体を `shell: bash` (pipefail) にした
- `fuzz.yml` の 3 target は毎回 compile に失敗していた (`alice_sdf::compiled` / `alice_sdf::prelude` の E0432) fuzz job が lib を build するのに、lib が compile する sibling (`alice-sdf`) まで空の stub にしていたため job に `continue-on-error: true` があったので run は success と表示されていた ci.yml と同じく `ALICE-SDF` / `ALICE-LOL` / `ALICE-Zip` を実 checkout し、stub は `make-stubs.py` (flag 無し、compile される dep の stub は拒否) で任意の sibling だけに作り、`continue-on-error` を外した (crash と build の失敗は job を落とす) `Cargo.toml` と `.github/make-stubs.py` の変更でも fuzz が走るように paths に足し、run step は `shell: bash` (pipefail) で動かす
- `alice-physics` の version 要求を `1.4` から `2.0` に、`alice-db` を `0.2.0-beta.3` から `0.3.0-beta.2` に追従 どちらも実 crate が先に進んでいて `cargo metadata` が解決できない状態だった 2.0.0 の破壊的変更 (`#[non_exhaustive]` の一括付与と enum の variant 追加) に当たる呼び出しは無く、`BodyType` の 3 variant も変わっていないので `physics_bridge` の `match` はそのまま通る
- `db` feature が `alice-db/fs` を有効化するようにした `AliceDB::open` (path 指定の file backend) は `fs` feature 配下なので、`default-features = false` のままでは in-memory backend しか無く `db_bridge` が compile できない 実測: `cargo check --lib --features db` / `cargo test --lib --features physics,db` (146 件) が exit 0
- `alice-analytics` の version 要求を `0.3` に追従 実 crate が 0.3.0 になったので `^0.2` は解決できない 0.3.0 は `alice-det-math` を `^0.4` に上げた版で、`analytics_bridge` が使う `anomaly::MadDetector` と `sketch::{CountMinSketch, DDSketch, HyperLogLog}` は API が変わっていない `.github/actions/alice-stubs/action.yml` の stub version も 0.3.0 に揃えた (この action を使うのは `fuzz.yml` と `security-audit.yml` で、`ci.yml` は `make-stubs.py` が `Cargo.toml` から導出する)
- `alice-analytics` の version 要求を `0.2` に追従 実 crate が 0.2.0 になったので `^0.1.1` は解決できず、`cargo metadata` が `failed to select a version for the requirement alice-analytics = "^0.1.1"` で落ちていた (feature 無効でも optional dep は lock に載るので、`analytics` を使わない build も止まる) `.github/actions/alice-stubs/action.yml` の stub version も 0.2.0 に揃えた (`ci.yml` 側は `make-stubs.py` が `Cargo.toml` から導出するので追従不要)
- `analytics_bridge` が compile できない状態だったのを修正 `alice_analytics::prelude` を import していたが、依存 crate の module 構成が変わった際に `prelude` は無くなっていた 型自体は実在するので `anomaly::MadDetector` と `sketch::{CountMinSketch, DDSketch, HyperLogLog}` から直接 import する形にした
- CI 設定と manifest の comment を書き直し、外部参照でなく理由そのものを書くようにした (`.github/` 5 file / `Cargo.toml` / `deny.toml` / `fuzz/` 4 file)
- CI の test job が `alice-lol` の非 optional 依存 `alice-zip` を checkout しておらず、stub 生成の段で止まっていた

### Added
- CI に `cargo check --lib --features analytics` を追加し、`ALICE-Analytics` を real checkout するようにした `analytics` は optional feature なので既定 build では 1 行も compile されず、上の `prelude` 不整合はそのために見逃されていた stub は空 crate なので型を名指しする bridge には使えず、`make-stubs.py` は既に存在する sibling を飛ばすので real checkout と併用できる
- path dep 5 個 (`alice-sdf` / `alice-lol` / `alice-analytics` / `alice-physics` / `alice-db`) に `version` を明記 `path` だけだと `cargo package` が `all dependencies must have a version requirement specified when packaging` で落ちて publish できない publish 時は `path` が外れて version 要求だけが残るので、値は相手の実 version に合わせた 通常の local build は `path` 優先のまま変わらない なお本 crate の publish は依然として不可 — `alice-sdf 4.0.0` / `alice-lol 0.4.0` が crates.io 未公開 (それぞれ 3.1.0 / 0.3.0 まで) で `cargo package` が解決に失敗する (cargo 1.98.1 は packaging 時に path dep の実在を crates.io に対して検証する) そのため `package-integrity` job は入れていない (恒久 red を置かない)
- CI の stub 生成 2 箇所 (`.github/workflows/ci.yml` / `.github/actions/alice-stubs/action.yml`) の stub version を実 crate に追従 上の version 追加で `alice-sdf 1.7.4` / `alice-physics 0.12.0` / `alice-analytics 0.1.0` / `alice-db 0.1.0` の stub が要求を外し、CI が resolve 段階で落ちる状態だった (optional dep も lock に載るので feature 無効でも解決される) 実測で確認: stub を 0.12.0 に戻すと `failed to select a version for the requirement alice-physics = "^1.4"` で fail、1.4.0 なら `cargo build --lib` 完走
- `.cargo/config.toml` の comment が local opt-in 手段として案内していた `.cargo/config.local.toml` は cargo が自動では読まない (2026-09-29 実測、置いても無言で無視される) 実際に効く `RUSTFLAGS="-C target-cpu=native" cargo bench` と `cargo bench --config 'build.rustflags=["-C","target-cpu=native"]'` の 2 経路に差し替え `target-cpu=native` を置かない方針そのものは commit 2233dc0 から変更なし (ALICE-LLM / ALICE-Text と文面を揃えた)

## [0.3.0] — 2026-02-28

### Added
- **Adaptive Quality** — Distance-based epsilon scaling and step over-relaxation for faster rendering of heavy SDF scenes
- **Quality Presets** — Fast / Balanced / Quality / Ultra one-click presets for raymarching settings
- **Adaptive AO** — Ambient occlusion sample count reduced for distant surfaces (5 → 2 when `t > 20.0`)
- Quality Preset combo box and Adaptive Quality checkbox in SDF panel
- `QualityPreset` enum with `apply_quality_preset()` for programmatic preset application
- `quality_flags` uniform field (replaces `_pad3` at offset 92) for GPU-side feature toggles
- 8 new unit tests for `QualityPreset` and adaptive quality defaults

## [0.2.1] — 2026-02-28

### Changed
- Clippy pedantic 0 warnings across all targets (lib + binaries)
- Added `#![allow]` blocks to binary crate roots for package-wide consistency
- Changed `&self` to `self` on small Copy types (`AliceContentType::name`, `ExportFormat::extension/filter_name`, `SdfScene::name`)
- Fixed `alice-create` format strings, clone efficiency, long literal separators, doc backticks
- Fixed wildcard import in `alice-create` to explicit imports
- Added `#[allow]` annotations on stub methods (`asp::process_packet`, `ui::handle_event`)
- Fixed `physics_bridge.rs` test imports

## [0.2.0] — 2026-02-23

### Added
- **SDF 3D raymarching** — wgpu pipeline with WGSL shaders for real-time SDF rendering
- **ASDF format loader** — `.asdf`, `.asdf.json`, `.json` SDF scene files
- **Camera3D** — Orbit, dolly, pan with `#[inline(always)]` hot methods
- **SDF panel** — UI controls for 3D scene parameters, mesh export (GLB/OBJ)
- **X-Ray overlays** — MotionVectors, FftHeatmap, EquationOverlay, Wireframe
- **File info panel** — Metadata display with compression ratio
- **Stats collector** — Zero-alloc ring buffer for frame timing (O(1) per frame)
- **Export** — GLB and OBJ mesh export from SDF scenes
- **ViewerConfig constructors** — `for_fractal()`, `minimal()`, `for_sdf_file()`, `for_temperature_data()`
- **Bridge modules** — `analytics_bridge`, `physics_bridge`, `db_bridge` (feature-gated)
- **alice-create binary** — CLI tool for creating `.alice` files (linear, polynomial, fractal, Perlin, demo)

### Changed
- Migrated to wgpu 0.19 / winit 0.29 / egui 0.27
- Async I/O via tokio `spawn_blocking` for non-blocking file loading

## [0.1.0] — 2026-01-15

### Added
- **Procedural 2D rendering** — Perlin noise, fractals (Mandelbrot, Julia, BurningShip, Tricorn), linear/polynomial sensor data
- **ALICE format decoder** — `.alice` binary format with header, payload, metadata, CRC32
- **ALZ format decoder** — Zip archive format
- **egui UI** — Viewport controls, stats overlay, file dialog
- **Infinite zoom** — Procedural zoom with LOD adaptation
- **Screenshot** — F12 to capture PNG
- Release profile: `opt-level=3`, `lto=fat`, `codegen-units=1`, `strip=true`, `panic=abort`
- 68 unit tests
