//! Fuzz target: `AliceMetadata::parse(&[u8])` が任意 byte 列で panic しないことを検証
//!
//! .alice file の tail JSON metadata section を fuzz する:
//! - serde_json parse crash (unicode escape / recursion / large numbers)
//! - length prefix 悪用による Vec::with_capacity panic
//! - non-UTF8 byte 列で from_utf8 panic
//!
//! JSON payload は攻撃者制御 file の最内層で最も攻撃面が広い
//!
//! canonical CI template [[reference_alice_ci_canonical_template]] 準拠

#![no_main]

use alice_view::decoder::alice::AliceMetadata;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // parse は Result を返す
    // panic せずに Err / Ok のどちらかで復帰することを検証
    let _ = AliceMetadata::parse(data);
});
