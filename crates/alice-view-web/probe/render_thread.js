// what comes before the module is ready waits in a queue
const queued = [];
self.onmessage = (e) => queued.push(e.data);
const { default: init, probe_render_thread } = await import("./pkg/alice_view_web.js");
await init();
probe_render_thread(queued);
