# alice-view-web

A renderer that lives in a Web Worker and draws on an `OffscreenCanvas` with WebGPU.

- The page hands its canvas to the worker (`page::RenderHost::start`), forwards the canvas's size, and keeps a
  mirror of the state the worker posts every frame: `RenderHost::mirror()` is a plain read, with no round trip.
- The worker gets a device (`worker::gpu`) or the reason there is none (`NoWebGpu`), so the page can draw another
  way (WebGL2). Pipelines are made with `createRenderPipelineAsync` (`Gpu::pipeline`): the browser compiles them
  off its GPU thread, and frames drawn meanwhile with other pipelines keep coming.
- WebGPU is called through `js_sys::Reflect`, so a crate that builds this one needs no unstable cfg.

The crate has its own workspace and depends only on `wasm-bindgen`, `js-sys` and `web-sys`, so it does not pull the
viewer's native dependencies into a page.

## Checks

`probe/` holds a page and a worker built from the crate with feature `probe`, and `probe/check.mjs`, which drives
them in a headless Chromium:

| check | what it shows |
|---|---|
| a WebGPU device came in the worker | `worker::gpu` on a transferred canvas |
| the slow pipeline was created, the worker drew frames while it compiled, no gap over 50 ms meanwhile | `Gpu::pipeline` does not hold up the worker's frames |
| the mirror is read at once and follows the worker | `RenderHost::mirror` |
| without WebGPU the worker says why | `NoWebGpu` and its message |

In CI and on a machine whose browser has no GPU WebGPU, the device is SwiftShader, a software implementation. There
the device is lost ("destroyed") while a second pipeline compiles asynchronously in a worker, with this crate and with
a plain-JS worker alike, so CI sets `PROBE_ASYNC=skip`: the device, the mirror, the fallback and the size are checked,
and the async pipeline (the worker drawing while it compiles) is checked where the device stays: in CI on the same
SwiftShader with Chromium's Vulkan flags (`--use-angle=vulkan --disable-vulkan-surface`) over Mesa's software Vulkan
(lavapipe; without it the same lane lost its device, so the step is load-bearing). `probe/gpuinfo.mjs` prints what
Chromium then uses (WebGL renderer, WebGPU adapter), the lane asserts the adapter and runs 20 times, and on a real GPU (`--use-angle=vulkan --disable-vulkan-surface`
on an NVIDIA Ampere: 10 of 10, a 200-213 ms compile with the worker's frames at most 17 ms apart).

`probe/size.py` is the size gate: the crate's code in a browser (built without function names, `strip`), gzip -9, at most 50 KB of a page's 500 KB arrival
budget (an upper bound: the probe calls every part of the API). It fails when it compares nothing.

```sh
probe/build.sh                                   # wasm32 build + wasm-bindgen into probe/pkg
python3 probe/size.py                            # the size gate
python3 -m http.server -d probe 8000 &           # then
node probe/check.mjs http://127.0.0.1:8000/index.html webgpu
node probe/check.mjs http://127.0.0.1:8000/index.html none
```

Licence: LicenseRef-Proprietary, as the rest of this repository.
