//! The worker's side: a WebGPU device on the transferred canvas, pipelines that compile without
//! blocking, a full-screen pass, and the state posted to the page each frame.

use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{DedicatedWorkerGlobalScope, OffscreenCanvas, WebGl2RenderingContext};

/// Why there is no WebGPU here. Every case but [`NoWebGpu::NoContext`] comes before the canvas is asked for a
/// `webgpu` context, so [`webgl2`] can still draw on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoWebGpu {
    /// `navigator.gpu` is absent (no WebGPU in this browser, or not in workers).
    NoApi,
    /// The browser has WebGPU but gave no adapter: none (blocked GPU, or no GPU), or the request was refused, with its
    /// words.
    NoAdapter(String),
    /// The adapter gave no device.
    NoDevice(String),
    /// The canvas gave no `webgpu` context.
    NoContext,
}

impl NoWebGpu {
    /// The reason in words, without the formatting machinery
    #[must_use]
    pub fn words(&self) -> String {
        match self {
            Self::NoApi => String::from("no WebGPU in this browser"),
            Self::NoAdapter(e) if e.is_empty() => String::from("WebGPU gave no adapter"),
            Self::NoAdapter(e) => String::from("WebGPU gave no adapter: ") + e,
            Self::NoDevice(e) => String::from("WebGPU gave no device: ") + e,
            Self::NoContext => String::from("the canvas gave no webgpu context"),
        }
    }
}

impl core::fmt::Display for NoWebGpu {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoApi => write!(f, "no WebGPU in this browser"),
            Self::NoAdapter(e) if e.is_empty() => write!(f, "WebGPU gave no adapter"),
            Self::NoAdapter(e) => write!(f, "WebGPU gave no adapter: {e}"),
            Self::NoDevice(e) => write!(f, "WebGPU gave no device: {e}"),
            Self::NoContext => write!(f, "the canvas gave no webgpu context"),
        }
    }
}

/// A device and the canvas it draws on.
pub struct Gpu {
    /// The adapter the device came from, kept for as long as the device is used: once nothing holds it, the
    /// browser may let the instance go and later calls fail ("a valid external Instance reference no longer exists")
    pub adapter: JsValue,
    /// `navigator.gpu` itself, held for the same reason
    pub api: JsValue,
    /// What the browser says of the adapter (`GPUAdapterInfo`: vendor, architecture, description)
    pub adapter_info: JsValue,
    pub device: JsValue,
    pub queue: JsValue,
    pub context: JsValue,
    /// The canvas's preferred texture format, which pipelines draw in.
    pub format: JsValue,
    pub canvas: OffscreenCanvas,
}

fn get(o: &JsValue, k: &str) -> JsValue {
    Reflect::get(o, &k.into()).unwrap_or(JsValue::UNDEFINED)
}

fn call(o: &JsValue, method: &str, args: &[&JsValue]) -> Result<JsValue, JsValue> {
    let f: Function = get(o, method)
        .dyn_into()
        .map_err(|_| JsValue::from_str(&(String::from("no ") + method)))?;
    let a = Array::new();
    for x in args {
        a.push(x);
    }
    f.apply(o, &a)
}

fn obj(pairs: &[(&str, &JsValue)]) -> Object {
    let o = Object::new();
    for (k, v) in pairs {
        let _ = Reflect::set(&o, &(*k).into(), v);
    }
    o
}

/// What a thrown value says: its text, or an error's message (without the formatting machinery a `{:?}` brings)
#[must_use]
pub fn describe(v: &JsValue) -> String {
    v.as_string()
        .or_else(|| get(v, "message").as_string())
        .unwrap_or_else(|| String::from("(no message)"))
}

/// The worker's own scope.
///
/// # Panics
///
/// When called outside a dedicated worker.
/// The other way to draw on `canvas` when there is no WebGPU: a WebGL2 context, in the worker, on the same canvas.
/// `None` when the browser has no WebGL2 either, or the canvas already has another context (after
/// [`NoWebGpu::NoContext`]).
#[must_use]
pub fn webgl2(canvas: &OffscreenCanvas) -> Option<WebGl2RenderingContext> {
    canvas.get_context("webgl2").ok().flatten()?.dyn_into().ok()
}

#[must_use]
pub fn scope() -> DedicatedWorkerGlobalScope {
    js_sys::global().unchecked_into()
}

/// A WebGPU device and a configured context on `canvas`, or why there is none.
///
/// # Errors
///
/// [`NoWebGpu`] says which step gave nothing.
pub async fn gpu(canvas: &OffscreenCanvas) -> Result<Gpu, NoWebGpu> {
    let api = get(&scope().navigator().into(), "gpu");
    if api.is_undefined() || api.is_null() {
        return Err(NoWebGpu::NoApi);
    }
    let adapter = match call(&api, "requestAdapter", &[]) {
        Ok(p) => JsFuture::from(Promise::from(p))
            .await
            .map_err(|e| NoWebGpu::NoAdapter(describe(&e)))?,
        Err(e) => return Err(NoWebGpu::NoAdapter(describe(&e))),
    };
    if adapter.is_null() || adapter.is_undefined() {
        return Err(NoWebGpu::NoAdapter(String::new()));
    }
    let device = match call(&adapter, "requestDevice", &[]) {
        Ok(p) => JsFuture::from(Promise::from(p))
            .await
            .map_err(|e| NoWebGpu::NoDevice(describe(&e)))?,
        Err(e) => return Err(NoWebGpu::NoDevice(describe(&e))),
    };
    let context = canvas
        .get_context("webgpu")
        .ok()
        .flatten()
        .ok_or(NoWebGpu::NoContext)?;
    let format = call(&api, "getPreferredCanvasFormat", &[]).map_err(|_| NoWebGpu::NoContext)?;
    let config = obj(&[
        ("device", &device),
        ("format", &format),
        ("alphaMode", &"premultiplied".into()),
    ]);
    call(&context, "configure", &[&config]).map_err(|_| NoWebGpu::NoContext)?;
    Ok(Gpu {
        adapter_info: get(&adapter, "info"),
        adapter: adapter.clone(),
        api: api.clone(),
        queue: get(&device, "queue"),
        device,
        context: context.into(),
        format,
        canvas: canvas.clone(),
    })
}

impl Gpu {
    /// Call `report(kind, text)` when the device is lost (`kind` "lost") or reports an error no scope caught
    /// (`kind` "uncaptured"): what would otherwise only stop the frames without a word
    pub fn on_trouble(&self, report: impl Fn(&str, String) + 'static) {
        let report = std::rc::Rc::new(report);
        let r = report.clone();
        let lost = Closure::once(move |info: JsValue| {
            let reason = get(&info, "reason").as_string().unwrap_or_default();
            let message = get(&info, "message").as_string().unwrap_or_default();
            r("lost", reason + ": " + &message);
        });
        if let Ok(p) = get(&self.device, "lost").dyn_into::<Promise>() {
            let _ = p.then(&lost);
        }
        lost.forget();
        let r = report;
        let uncaptured = Closure::<dyn Fn(JsValue)>::new(move |e: JsValue| {
            let err = get(&e, "error");
            r(
                "uncaptured",
                get(&err, "message")
                    .as_string()
                    .unwrap_or_else(|| describe(&err)),
            );
        });
        let _ = call(
            &self.device,
            "addEventListener",
            &[&"uncapturederror".into(), uncaptured.as_ref()],
        );
        uncaptured.forget();
    }

    /// A render pipeline for a full-screen pass from WGSL (`vs` and `fs` entry points, layout from
    /// the shader), created with `createRenderPipelineAsync`: the browser compiles it off its GPU
    /// thread, and frames drawn meanwhile with other pipelines are not held up.
    ///
    /// # Errors
    ///
    /// The shader's or the pipeline's error, as the browser gives it.
    pub async fn pipeline(&self, wgsl: &str, vs: &str, fs: &str) -> Result<JsValue, JsValue> {
        let module = call(
            &self.device,
            "createShaderModule",
            &[&obj(&[("code", &wgsl.into())])],
        )?;
        let target = obj(&[("format", &self.format)]);
        let desc = obj(&[
            ("layout", &"auto".into()),
            (
                "vertex",
                &obj(&[("module", &module), ("entryPoint", &vs.into())]),
            ),
            (
                "fragment",
                &obj(&[
                    ("module", &module),
                    ("entryPoint", &fs.into()),
                    ("targets", &Array::of1(&target)),
                ]),
            ),
            ("primitive", &obj(&[("topology", &"triangle-list".into())])),
        ]);
        JsFuture::from(Promise::from(call(
            &self.device,
            "createRenderPipelineAsync",
            &[&desc],
        )?))
        .await
    }

    /// A uniform buffer holding `bytes` (a multiple of 16), and the bind group that gives it to
    /// `pipeline` at group 0, binding 0.
    ///
    /// # Errors
    ///
    /// The browser's error.
    pub fn uniforms(
        &self,
        pipeline: &JsValue,
        bytes: &[u8],
    ) -> Result<(JsValue, JsValue), JsValue> {
        // GPUBufferUsage.UNIFORM | COPY_DST
        let usage = JsValue::from(0x40 | 0x08);
        let size = JsValue::from(bytes.len().div_ceil(16) * 16);
        let buffer = call(
            &self.device,
            "createBuffer",
            &[&obj(&[("size", &size), ("usage", &usage)])],
        )?;
        self.write(&buffer, bytes)?;
        let layout = call(pipeline, "getBindGroupLayout", &[&0.into()])?;
        let entry = obj(&[
            ("binding", &0.into()),
            ("resource", &obj(&[("buffer", &buffer)])),
        ]);
        let group = call(
            &self.device,
            "createBindGroup",
            &[&obj(&[
                ("layout", &layout),
                ("entries", &Array::of1(&entry)),
            ])],
        )?;
        Ok((buffer, group))
    }

    /// Write `bytes` into `buffer` from its start.
    ///
    /// # Errors
    ///
    /// The browser's error.
    pub fn write(&self, buffer: &JsValue, bytes: &[u8]) -> Result<(), JsValue> {
        call(
            &self.queue,
            "writeBuffer",
            &[buffer, &0.into(), &Uint8Array::from(bytes)],
        )
        .map(|_| ())
    }

    /// Draw one full-screen triangle with `pipeline` (and `group` at 0, if any) onto the canvas.
    ///
    /// # Errors
    ///
    /// The browser's error.
    pub fn draw(&self, pipeline: &JsValue, group: Option<&JsValue>) -> Result<(), JsValue> {
        let texture = call(&self.context, "getCurrentTexture", &[])?;
        let view = call(&texture, "createView", &[])?;
        let clear = obj(&[
            ("r", &0.into()),
            ("g", &0.into()),
            ("b", &0.into()),
            ("a", &1.into()),
        ]);
        let attachment = obj(&[
            ("view", &view),
            ("loadOp", &"clear".into()),
            ("storeOp", &"store".into()),
            ("clearValue", &clear),
        ]);
        let encoder = call(&self.device, "createCommandEncoder", &[])?;
        let pass = call(
            &encoder,
            "beginRenderPass",
            &[&obj(&[("colorAttachments", &Array::of1(&attachment))])],
        )?;
        call(&pass, "setPipeline", &[pipeline])?;
        if let Some(g) = group {
            call(&pass, "setBindGroup", &[&0.into(), g])?;
        }
        call(&pass, "draw", &[&3.into()])?;
        call(&pass, "end", &[])?;
        let commands = call(&encoder, "finish", &[])?;
        call(&self.queue, "submit", &[&Array::of1(&commands)]).map(|_| ())
    }

    /// Follow a new size of the canvas's box.
    pub fn resize(&self, width: u32, height: u32) {
        self.canvas.set_width(width.max(1));
        self.canvas.set_height(height.max(1));
    }
}

/// Post the worker's state for this frame; the page's mirror holds the last one.
///
/// # Errors
///
/// When `state` cannot be cloned to the page.
pub fn post_state(state: &JsValue) -> Result<(), JsValue> {
    scope().post_message(&obj(&[("state", state)]))
}

/// Post anything else to the page (`{ kind, ... }` as the product defines it).
///
/// # Errors
///
/// When `message` cannot be cloned to the page.
pub fn post(message: &JsValue) -> Result<(), JsValue> {
    scope().post_message(message)
}

/// Wait `ms` milliseconds on the worker's own clock, whether or not any frame is presented
pub async fn sleep(ms: i32) {
    let p = Promise::new(&mut |resolve, _| {
        let _ = scope().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    let _ = JsFuture::from(p).await;
}

/// The next animation frame of the worker (its own `requestAnimationFrame`, which follows the
/// canvas's display).
pub async fn frame() -> f64 {
    let p = Promise::new(&mut |resolve, _| {
        let f = Closure::once_into_js(move |t: f64| {
            let _ = resolve.call1(&JsValue::NULL, &t.into());
        });
        let _ = call(&scope().into(), "requestAnimationFrame", &[&f]);
    });
    JsFuture::from(p)
        .await
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}
