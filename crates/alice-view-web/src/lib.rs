//! A renderer that lives in a render thread and draws on an `OffscreenCanvas`, with WebGPU.
//!
//! Why a render thread: creating a WebGL or WebGPU context and compiling shaders on the page's own thread
//! stops the page (a 100-300 ms task on a slow machine). Here the page keeps the DOM and the input;
//! the render thread owns the GPU. Why `createRenderPipelineAsync`: the browser compiles the pipeline off
//! its GPU thread, so frames keep coming while a large shader compiles.
//!
//! - [`page`]: on the page. Hands the canvas to a render thread, forwards its size, and keeps a mirror of
//!   the state the render thread posts every frame, so the page can read it at once, without a round trip.
//! - [`render_thread`]: in the render thread. Gets a WebGPU device from the canvas, or says why it cannot and gives a
//!   WebGL2 context on the same canvas instead ([`render_thread::webgl2`]), creates pipelines without blocking,
//!   draws a full-screen pass, and posts its state.
//!
//! WebGPU is called through `js_sys::Reflect`, not web-sys's WebGPU types, which need the unstable
//! `web_sys_unstable_apis` cfg in every crate that builds this one.
//!
//! Author: Moroya Sakamoto

pub mod page;
pub mod render_thread;

#[cfg(feature = "probe")]
pub mod probe;
