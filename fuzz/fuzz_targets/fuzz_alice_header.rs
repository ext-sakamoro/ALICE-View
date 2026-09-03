//! Fuzz target: `AliceHeader::parse(&[u8])` が任意 byte 列で panic しないことを検証
//!
//! .alice file の 24-byte header 単体を fuzz する:
//! - magic bytes 不正 → Err
//! - version / content_type 不正 → Err
//! - trailing bytes / trunc 時の panic ゼロ
//!
//! header parse は attack surface の最外周なので、malformed input が
//! 内部 slice 演算で panic することを完全に防ぐ必要がある
//!
//! canonical CI template [[reference_alice_ci_canonical_template]] 準拠

#![no_main]

use alice_view::decoder::alice::AliceHeader;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // parse は Result を返す
    // panic せずに Err / Ok のどちらかで復帰することを検証
    let _ = AliceHeader::parse(data);
});
