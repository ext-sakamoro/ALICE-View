//! A page and a worker that exercise the crate in a real browser: `probe/check.mjs` reads what they
//! report. Not part of what a product links (feature `probe`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[allow(unused_imports)]
use js_sys::Function;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, MessageEvent, OffscreenCanvas};

use crate::page::RenderHost;
use crate::worker::{self, Gpu};

thread_local! {
    static HOST: RefCell<Option<RenderHost>> = const { RefCell::new(None) };
}

const QUICK: &str = "
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let p = array(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
    return vec4f(p[i], 0.0, 1.0);
}
@fragment fn fs(@builtin(position) q: vec4f) -> @location(0) vec4f {
    return vec4f(fract(q.x / 64.0), 0.2, 0.4, 1.0);
}
";

/// A shader that takes the compiler a while: many unrolled steps of a little SDF march
fn slow_shader(steps: u32) -> String {
    // the digits of k, without the formatting machinery (the probe counts toward the size gate)
    let digits = |mut k: u32| {
        let mut d = Vec::new();
        loop {
            d.push(b'0' + (k % 10) as u8);
            k /= 10;
            if k == 0 {
                break;
            }
        }
        d.reverse();
        String::from_utf8(d).unwrap_or_default()
    };
    let mut out = String::from(QUICK.split("@fragment").next().unwrap_or_default());
    out.push_str("\n@fragment fn fs(@builtin(position) q: vec4f) -> @location(0) vec4f {\n    var p = vec3f(q.xy / 512.0, -2.0);\n    var d = 1e9;\n    let t = q.x * 0.01;\n");
    for k in 0..steps {
        let n = digits(k);
        for part in [
            "    d = min(d, length(p - vec3f(sin(t + ",
            &n,
            ".0), cos(t * 1.",
            &n,
            "), 0.",
            &n,
            ")) - 0.2);\n    p = p + vec3f(0.0, 0.0, d * 0.5);\n",
        ] {
            out.push_str(part);
        }
    }
    out.push_str("    return vec4f(d, d * 0.5, 0.2, 1.0);\n}\n");
    out
}

fn obj(pairs: &[(&str, JsValue)]) -> Object {
    let o = Object::new();
    for (k, v) in pairs {
        let _ = Reflect::set(&o, &(*k).into(), v);
    }
    o
}

/// The page: hand `#c` to `worker.js`, and keep the mirror readable as `window.__probeMirror()`.
#[wasm_bindgen]
pub fn probe_page(steps: u32) -> Result<(), JsValue> {
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;
    let canvas: HtmlCanvasElement = doc.get_element_by_id("c").ok_or("no #c")?.dyn_into()?;
    let host = RenderHost::start(
        &canvas,
        "./worker.js",
        &obj(&[("steps", steps.into())]).into(),
    )?;
    let said = Rc::new(RefCell::new(Array::new()));
    let times = Rc::new(RefCell::new(Array::new()));
    {
        let (said, times) = (said.clone(), times.clone());
        host.listen(move |m| {
            match Reflect::get(m, &"state".into()) {
                // every frame's time, to find the gaps between the worker's frames
                Ok(s) if !s.is_undefined() => {
                    let _ = Reflect::get(&s, &"t".into()).map(|t| times.borrow().push(&t));
                }
                _ => {
                    said.borrow().push(m);
                }
            }
        });
    }
    let window = web_sys::window().ok_or("no window")?;
    let mirror = Closure::wrap(Box::new(|| {
        HOST.with_borrow(|h| h.as_ref().map_or(JsValue::NULL, RenderHost::mirror))
    }) as Box<dyn Fn() -> JsValue>);
    Reflect::set(&window, &"__probeMirror".into(), mirror.as_ref())?;
    mirror.forget();
    let told =
        Closure::wrap(Box::new(move || said.borrow().clone().into()) as Box<dyn Fn() -> JsValue>);
    Reflect::set(&window, &"__probeSaid".into(), told.as_ref())?;
    told.forget();
    let frames =
        Closure::wrap(Box::new(move || times.borrow().clone().into()) as Box<dyn Fn() -> JsValue>);
    Reflect::set(&window, &"__probeTimes".into(), frames.as_ref())?;
    frames.forget();
    HOST.with_borrow_mut(|h| *h = Some(host));
    Ok(())
}

async fn run(canvas: OffscreenCanvas, steps: u32) -> Result<(), JsValue> {
    let gpu: Rc<Gpu> = match worker::gpu(canvas).await {
        Ok(g) => Rc::new(g),
        Err(no) => {
            return worker::post(
                &obj(&[("kind", "fallback".into()), ("reason", no.words().into())]).into(),
            );
        }
    };
    // what the device says when it is lost or reports an error nobody caught: printed, never silent
    gpu.on_trouble(|kind, text| {
        let _ = worker::post(&obj(&[("kind", kind.into()), ("error", text.into())]).into());
    });
    let quick = gpu.pipeline(QUICK, "vs", "fs").await?;
    // which adapter: on a machine without a GPU's WebGPU it is a software one, which shows the mechanism, not GPU time
    let info = &gpu.adapter_info;
    let field = |k: &str| {
        Reflect::get(info, &k.into())
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    };
    let adapter = field("vendor") + " " + &field("architecture") + " " + &field("description");
    worker::post(
        &obj(&[
            ("kind", "device".into()),
            ("adapter", adapter.trim().into()),
        ])
        .into(),
    )?;
    let heavy_done = Rc::new(Cell::new(-1.0_f64));
    let mut frame = 0u32;
    let mut asked = 0.0;
    // the state goes to the page from its own beat, every 16 ms, whether or not a frame is drawn: the mirror does not
    // depend on the GPU (frame, its time, when the slow pipeline was asked, when it came)
    let shown = Rc::new(Cell::new((0u32, 0.0f64, 0.0f64)));
    {
        let (shown, heavy) = (shown.clone(), heavy_done.clone());
        wasm_bindgen_futures::spawn_local(async move {
            for tick in 1..=1200u32 {
                worker::sleep(16).await;
                let (frame, t, asked) = shown.get();
                let _ = worker::post_state(
                    &obj(&[
                        ("tick", tick.into()),
                        ("frame", frame.into()),
                        ("t", t.into()),
                        ("asked", asked.into()),
                        ("heavy", heavy.get().into()),
                    ])
                    .into(),
                );
            }
        });
    }
    loop {
        let t = worker::frame().await;
        gpu.draw(&quick, None)?;
        frame += 1;
        // (steps 0: no slow pipeline, where the adapter cannot make one in a worker)
        if frame == 10 && steps > 0 {
            // compile a slow pipeline while the quick one keeps drawing
            asked = t;
            let (shared, done) = (gpu.clone(), heavy_done.clone());
            let wgsl = slow_shader(steps);
            wasm_bindgen_futures::spawn_local(async move {
                let made = shared.pipeline(&wgsl, "vs", "fs").await;
                if let Err(e) = &made {
                    let _ = worker::post(
                        &obj(&[
                            ("kind", "heavyError".into()),
                            ("error", worker::describe(e).into()),
                        ])
                        .into(),
                    );
                }
                let ok = made.is_ok();
                let now = js_sys::Reflect::get(&worker::scope(), &"performance".into())
                    .ok()
                    .and_then(|p| js_sys::Reflect::get(&p, &"now".into()).ok().map(|f| (p, f)))
                    .and_then(|(p, f)| {
                        f.dyn_ref::<js_sys::Function>()
                            .and_then(|f| f.call0(&p).ok())
                    })
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                done.set(if ok { now } else { -2.0 });
            });
        }
        shown.set((frame, t, asked));
        if frame > 600 {
            return Ok(());
        }
    }
}

/// The worker: messages that came before the module was ready are in `queued`.
#[wasm_bindgen]
pub fn probe_worker(queued: Array) {
    let handle = |data: JsValue| {
        if Reflect::get(&data, &"kind".into())
            .ok()
            .and_then(|k| k.as_string())
            .as_deref()
            == Some("init")
        {
            let canvas: OffscreenCanvas = Reflect::get(&data, &"canvas".into())
                .unwrap_throw()
                .unchecked_into();
            let steps = Reflect::get(&data, &"init".into())
                .and_then(|i| Reflect::get(&i, &"steps".into()))
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(64.0) as u32;
            wasm_bindgen_futures::spawn_local(async move {
                if let Err(e) = run(canvas, steps).await {
                    let _ = worker::post(
                        &obj(&[
                            ("kind", "error".into()),
                            ("error", worker::describe(&e).into()),
                        ])
                        .into(),
                    );
                }
            });
        }
    };
    for d in queued.iter() {
        handle(d);
    }
    let f = Closure::wrap(
        Box::new(move |e: MessageEvent| handle(e.data())) as Box<dyn FnMut(MessageEvent)>
    );
    worker::scope().set_onmessage(Some(f.as_ref().unchecked_ref()));
    f.forget();
}
