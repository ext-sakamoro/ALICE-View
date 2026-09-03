//! Fuzz target: `AliceFile::parse(&[u8])` が任意 byte 列で panic しないことを検証
//!
//! 攻撃者制御 .alice file の parse で:
//! - length prefix 悪用による OOM allocation panic
//! - JSON metadata section の serde crash
//! - payload 種別 mismatch による内部 index OOB
//!
//! を全て `Result::Err` に落ちることを保証する
//!
//! canonical CI template [[reference_alice_ci_canonical_template]] 準拠

#![no_main]

use alice_view::decoder::alice::AliceFile;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // parse は Result を返す
    // panic せずに Err / Ok のどちらかで復帰することを検証
    let _ = AliceFile::parse(data);
});
