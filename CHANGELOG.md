# Changelog

All notable changes to ALICE-View are documented here.

## [Unreleased]

### Changed
- Licence: `MIT` → `LicenseRef-Proprietary`
- `src/shaders/{unity,godot,ue5}/README.md` と `CONTRIBUTING.md` の licence の節、`deny.toml` の comment を `LicenseRef-Proprietary` に合わせた
- 依存: sibling の ALICE-Physics を `50a12e1` に (deny の例外は `LicenseRef-Proprietary`)
- release.yml (Release Build): sibling を `SIBLING_REPOS` の 16 repo の `git clone … || true` (毎回 main、失敗を黙って捨てる、ALICE-LOL が無い) でなく、ci.yml の test job と同じ 4 repo (ALICE-SDF / ALICE-LOL / ALICE-Zip / ALICE-Analytics) を `.github/actions/siblings` の固定 commit から取り、残りの optional な path 依存は `make-stubs.py` の stub にする 取れない sibling は build を失敗させる `workflow_dispatch` に `dry_run` (build と test だけで release に upload しない) を足した `.github/actions/siblings` に取得先の dir を指定する `into` を足した `.github/check_sibling_pins.py` は workflow の `git` / `gh` の clone / fetch と `SIBLING_REPOS` も fail にする `make-stubs.py` は出力を UTF-8 で書く (Windows の runner の既定の cp1252 では日本語の出力で落ちていた) ci.yml で cp1252 の stdout のまま 1 回走らせる
- CI: ci.yml / fuzz.yml / security-audit.yml の sibling (ALICE-SDF / ALICE-LOL / ALICE-Zip / ALICE-Analytics / ALICE-Physics / ALICE-DB) は `.github/actions/siblings` から、その env の 1 箇所に固定した commit で取る (以前は fuzz と security-audit が毎回 main を checkout し、同じ commit でも sibling の main が動くと結果が変わった) 固定の commit が取れない・固定の無い repo を指定した時は失敗し、GitHub Actions の外では止まる `.github/check_sibling_pins.py` (試験 `.github/test_check_sibling_pins.py`) が、workflow に sibling の直接の checkout があるか、固定の無い repo を指定していたら CI を fail にする workflow は `ci/**` の branch でも走る
- `alice-physics` の path 依存を `../ALICE-Physics` から `../ALICE-Physics/crates/alice-physics` に移した (ALICE-Physics 4.0 で repository の root の package は `alice-physics-core` になり、`alice-physics` は `crates/alice-physics` の窓口の crate になった 旧 path では cargo が `alice-physics` を見つけられない) `deny.toml` の license の例外に `alice-physics-core` を足した
- `rfd` (ファイル選択の dialog) を 0.15 から 0.17 に上げた 0.15 が linux で引く ashpd 0.11 → zbus 5 の経路の `zbus_names` / `zvariant` / `zvariant_derive` が RustSec で unmaintained になり cargo-deny の advisories が失敗していた 0.17 はこの経路に依存しない 呼び出し (`rfd::FileDialog::new()` の 2 か所) は変更なし 実測: `cargo deny --all-features check` が advisories / bans / licenses / sources とも ok、`cargo test --all-features` が lib 148 / bin 122 / 統合 3 件
- `alice-analytics` の要求を `>=0.5, <0.7` から `>=0.6, <0.8` に広げた ALICE-Analytics の 0.6.0 と 0.7.0 (alice-zip 0.11 の型を `law` feature で受け渡す) のどちらの sibling でも build できる CI の ALICE-Analytics の checkout は 0.7.0 の commit (bf1abd52…) に固定した 使っている API に変更は無く code の変更は無い 実測: 0.6.0 (75ac058) と 0.7.0 (bf1abd5) の sibling のそれぞれで `cargo test --features analytics` が lib 141 / bin 122 / 統合 3 件で通る View を公開する前に下限を 0.7 に上げる (公開済みの 0.6.0 は alice-zip 0.10 の型を持つ)
- `alice-sdf` の要求を `"6"` から `">=6, <8"` に広げた (alice-sdf 7 の変更を受け取れるようにする、6.x でも build できる)
- `alice-physics` の要求を `"3.0"` から `">=3.0, <5"` に広げた (alice-physics 4.0.0 の変更は bridge の構築子・GPU bridge の trait・privacy の構築子で、この crate はどれも使わない 3.1.0 でも 4.0.0 でも build できる)
- `alice-analytics` の要求を `0.5` から `>=0.5, <0.7` に広げた ALICE-Analytics 0.6.0 の sibling に対しても build できる 使っている API に変更は無く code の変更は無い 実測: 0.6.0 の sibling で `cargo check --lib --features analytics` が要求 `^0.5` では解決できず、変更後は exit 0
- `alice-db` の要求を `0.4` から `0.5` に上げた View は sibling checkout の ALICE-DB 0.5.0 に対して build する crates.io への View の公開は alice-db 0.5.0 が registry に載るまで待つ 使っている API に変更は無く code の変更は無い 実測: `cargo clippy --all-targets --all-features -- -D warnings` が警告 0、`cargo test --all-features` が lib 148 / bin 122 / 統合 3 件で通る
- `alice-lol` の要求を `0.4` から `>=0.4.1, <0.6` に広げた 0.4.1 と 0.5.x のどちらでも解決する `.github/make-stubs.py` は `>=` と `<` の範囲要求から下限を導出するようにした (従来は範囲を stub 化できず loud fail)
- 依存の要求を公開版に追従した: `alice-sdf` `5.0` → `6`、`alice-analytics` `0.3` → `0.5`、`alice-physics` `2.0` → `3.0`、`alice-db` `0.3.0-beta.2` → `0.4` (どの crate も先に進んでいて、旧い要求では `cargo metadata` が解決できなかった) 使っている API に変更は無く、code の変更は無い 実測: `cargo build --lib`、`cargo test --all-features` (lib 148 / bin 122 / 統合 3 件) と `cargo check --lib --features analytics,physics,db` が exit 0、`cargo clippy --all-targets --all-features -- -D warnings` が警告 0
- `.github/actions/alice-stubs` が stub の版と feature を literal で書くのをやめ、`.github/make-stubs.py --allow-stub-all` に委譲するようにした これで版の出所は `Cargo.toml` 1 箇所になる (従来は `ci.yml` が導出、本 action が literal で、要求を動かすたびに 2 箇所の同期が要った) `--allow-stub-all` は「消費側の lib を既定 feature で compile しない job」(cargo audit / deny / semver-checks / fuzz) 専用で、flag を付けなければ従来どおり compile される dep への stub を拒否する
- `make-stubs.py` が `[features]` 表の `<dep>/<feat>` 記法 (`db = ["dep:alice-db", "alice-db/fs"]`) を stub の feature として拾うようにした 拾わないと `depends on alice-db with feature fs but alice-db does not have that feature` で resolve 段から落ちる
- `alice-sdf` の要求を `4.0` から `5.0` に上げた (5.0 で変わった API の利用は無く、code の変更は無い)

### Fixed
- `semver-checks` の基準が、自分の `Cargo.toml` の要求を満たさない sibling に当たって解決できなかった (基準の `alice-db = "0.4"` に対し sibling は 0.5.0 で、比較の前に `failed to select a version` で止まる) 基準の checkout にだけ、sibling が要求を満たさなくなった path 依存の `path` を外し、その要求を registry から解決する (`.github/semver_gate.py relax`) registry に無い版なら解決で失敗し、比較 0 件として fail のままである 試験 `Relax` を `.github/test_semver_gate.py` に足した
- `semver-checks` の比較の基準を直前の commit (HEAD~1) から変更前の状態に変えた 複数 commit の push で前の commit が API を変えると、HEAD~1 が既にその変更を含むので検出できなかった main への push は push 前の main (`github.event.before`、それが履歴に無いか HEAD の祖先でない時 (main を書き換えた時) は直前の commit を使い、理由を summary に出す)、pull request と branch はその merge base、定期実行は最後の release tag と比べる (`.github/semver_gate.py`) 判定も同じ script に移し、新しい major 版が要る変更は fail、新しい minor 版が要る変更は `Cargo.toml` の版が基準から変わっていなければ fail にし、文言を「requires a new minor / major version」にした 試験 `.github/test_semver_gate.py` (一時 repo での複数 commit の push、書き換えた main への push、pull request、dispatch、定期実行の基準と、判定 5 通り) を job の中で先に走らせる
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
