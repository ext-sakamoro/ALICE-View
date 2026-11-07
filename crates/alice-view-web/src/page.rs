//! The page's side: the canvas goes to a worker, and the worker's state comes back as a mirror.

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, MessageEvent, ResizeObserver, Worker, WorkerOptions, WorkerType};

/// What the worker sends: `{ state }` every frame, or anything else the product defines.
type Listener = Box<dyn FnMut(&JsValue)>;
/// The observer of the canvas's box and what it calls.
type Watch = (ResizeObserver, Closure<dyn FnMut(Array)>);

/// The worker that draws, and the last state it posted.
pub struct RenderHost {
    worker: Worker,
    mirror: Rc<RefCell<JsValue>>,
    listener: Rc<RefCell<Option<Listener>>>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _resize: Option<Watch>,
}

fn set(o: &Object, k: &str, v: &JsValue) -> Result<(), JsValue> {
    Reflect::set(o, &k.into(), v).map(|_| ())
}

/// The canvas's size in device pixels.
fn device_size(canvas: &HtmlCanvasElement) -> (u32, u32) {
    let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
    (
        ((f64::from(canvas.client_width()) * dpr) as u32).max(1),
        ((f64::from(canvas.client_height()) * dpr) as u32).max(1),
    )
}

impl RenderHost {
    /// Hand `canvas` to a module worker started from `script`, with `init` passed along as it is.
    /// The worker receives `{ kind: "init", canvas, width, height, init }` first (the canvas
    /// transferred), then `{ kind: "resize", width, height }` whenever the canvas's box changes.
    ///
    /// # Errors
    ///
    /// When the canvas cannot be transferred (it already has a context, or the browser has no
    /// `OffscreenCanvas`) or the worker cannot start.
    pub fn start(
        canvas: &HtmlCanvasElement,
        script: &str,
        init: &JsValue,
    ) -> Result<Self, JsValue> {
        let offscreen = canvas.transfer_control_to_offscreen()?;
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker = Worker::new_with_options(script, &options)?;
        let mirror = Rc::new(RefCell::new(JsValue::NULL));
        let listener: Rc<RefCell<Option<Listener>>> = Rc::new(RefCell::new(None));
        let on_message = {
            let (mirror, listener) = (mirror.clone(), listener.clone());
            Closure::wrap(Box::new(move |e: MessageEvent| {
                let data = e.data();
                if let Ok(state) = Reflect::get(&data, &"state".into()) {
                    if !state.is_undefined() {
                        *mirror.borrow_mut() = state;
                    }
                }
                if let Some(f) = listener.borrow_mut().as_mut() {
                    f(&data);
                }
            }) as Box<dyn FnMut(MessageEvent)>)
        };
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        let (w, h) = device_size(canvas);
        let first = Object::new();
        set(&first, "kind", &"init".into())?;
        set(&first, "canvas", &offscreen)?;
        set(&first, "width", &w.into())?;
        set(&first, "height", &h.into())?;
        set(&first, "init", init)?;
        worker.post_message_with_transfer(&first, &Array::of1(&offscreen))?;
        // the size follows the box; a missing ResizeObserver only means the size stays as it was
        let resize = {
            let (worker, watched) = (worker.clone(), canvas.clone());
            let f = Closure::wrap(Box::new(move |_entries: Array| {
                let (w, h) = device_size(&watched);
                let o = Object::new();
                let _ = set(&o, "kind", &"resize".into());
                let _ = set(&o, "width", &w.into());
                let _ = set(&o, "height", &h.into());
                let _ = worker.post_message(&o);
            }) as Box<dyn FnMut(Array)>);
            ResizeObserver::new(f.as_ref().unchecked_ref())
                .ok()
                .map(|o| {
                    o.observe(canvas.as_ref());
                    (o, f)
                })
        };
        Ok(Self {
            worker,
            mirror,
            listener,
            _on_message: on_message,
            _resize: resize,
        })
    }

    /// The last state the worker posted (`null` before the first): a plain read, no round trip.
    #[must_use]
    pub fn mirror(&self) -> JsValue {
        self.mirror.borrow().clone()
    }

    /// Send the worker a message (input, a change of what to draw).
    ///
    /// # Errors
    ///
    /// When the message cannot be cloned to the worker.
    pub fn post(&self, message: &JsValue) -> Result<(), JsValue> {
        self.worker.post_message(message)
    }

    /// Called with every message the worker sends, after the mirror is updated.
    pub fn listen(&self, f: impl FnMut(&JsValue) + 'static) {
        *self.listener.borrow_mut() = Some(Box::new(f));
    }

    /// Stop the worker.
    pub fn stop(&self) {
        self.worker.terminate();
    }
}
