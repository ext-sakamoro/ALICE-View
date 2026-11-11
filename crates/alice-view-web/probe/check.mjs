// What the crate claims, in a real browser: a WebGPU device in a render thread on the transferred canvas, a slow pipeline
// compiled while the render thread keeps drawing, the page's mirror following the render thread's state, and the reason given when
// there is no WebGPU.
//
//   node probe/check.mjs <url of probe/index.html> <webgpu|none>
//
// Author: Moroya Sakamoto
import { chromium } from "playwright";
import { isStall } from "./stall.mjs";

const [url, mode] = process.argv.slice(2);
const problems = [];
let compared = 0;
const check = (what, ok, detail = "") => {
  compared++;
  console.log(`${ok ? "ok  " : "FAIL"} ${what}${detail === "" ? "" : `: ${detail}`}`);
  if (!ok) problems.push(what);
};
const base = (process.env.PROBE_CHROME_ARGS ?? "").split(" ").filter(Boolean);
const args = mode === "webgpu" ? [...base, "--enable-unsafe-webgpu", "--enable-features=Vulkan"] : base.filter((a) => !/webgpu/i.test(a));
const browser = await chromium.launch({ headless: true, args });
const page = await browser.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
// PROBE_ASYNC=skip: the runner's adapter cannot create a pipeline asynchronously in a render thread (SwiftShader in Chromium:
// the device is destroyed while the second createRenderPipelineAsync runs, measured 2026-11-08 with this probe and
// with a plain-JS render thread alike, the device held on the render thread's global or not). The async pipeline is then not
// measured here; it is measured on a real GPU
const skipAsync = process.env.PROBE_ASYNC === "skip";
// PROBE_HOLD_MS: the render thread busy-waits this long after asking for the slow pipeline (the gate's teeth: a hold
// as long as the compile must count as a stall)
await page.goto(`${url}?steps=${skipAsync ? 0 : process.env.PROBE_STEPS ?? 96}&hold=${process.env.PROBE_HOLD_MS ?? 0}`);
const said = async () => page.evaluate(() => (window.__probeSaid ? window.__probeSaid() : []));
const kinds = async () => (await said()).map((m) => m.kind);
for (let i = 0; i < 300 && !(await kinds()).some((k) => k === "device" || k === "fallback" || k === "error"); i++) await page.waitForTimeout(100);
const k = await kinds();
if (mode === "webgpu") {
  const dev = (await said()).find((m) => m.kind === "device");
  check("a WebGPU device came in the render thread, and says its adapter", !!dev && typeof dev.adapter === "string" && dev.adapter.length > 0, JSON.stringify(dev));
  // PROBE_ADAPTER: the adapter this lane says it measures; another one fails (a lane must not measure what it does not name)
  if (process.env.PROBE_ADAPTER) check(`the adapter is the one this lane names (${process.env.PROBE_ADAPTER})`, !!dev && dev.adapter.toLowerCase().includes(process.env.PROBE_ADAPTER.toLowerCase()), dev ? dev.adapter : "none");
  let m = null;
  if (skipAsync) {
    console.log("not measured on this runner: the async pipeline (PROBE_ASYNC=skip, software adapter)");
    m = { asked: 0 };
  } else {
    // the slow pipeline is asked for at the 10th frame; wait for it
    for (let i = 0; i < 600; i++) {
      m = await page.evaluate(() => window.__probeMirror());
      if (m && m.heavy !== -1) break;
      await page.waitForTimeout(100);
    }
    const failed = (await said()).find((x) => x.kind === "heavyError");
    check("the slow pipeline was created", m && m.heavy > 0, failed ? failed.error : JSON.stringify(m));
    // a control window as long as the compile, after it (from 100 ms after the pipeline came): wait until the render
    // thread's frames have passed its end, so the same run says what gaps this machine shows with no compile
    const ctrl0 = m.heavy + 100;
    const ctrl1 = ctrl0 + (m.heavy - m.asked);
    for (let i = 0; i < 600; i++) {
      const last = await page.evaluate(() => { const x = window.__probeTimes(); return x.length ? x[x.length - 1] : 0; });
      if (last >= ctrl1) break;
      await page.waitForTimeout(100);
    }
    // the beat repeats a frame's time until the next frame: each frame once
    const t = [...new Set(await page.evaluate(() => window.__probeTimes()))];
    const during = [];
    for (let i = 1; i < t.length; i++) if (t[i - 1] >= m.asked && t[i] <= m.heavy + 100) during.push(t[i] - t[i - 1]);
    // the frame period before the ask (the median of those intervals): what "a frame" is on this runner
    const before = [];
    for (let i = 1; i < t.length; i++) if (t[i] < m.asked) before.push(t[i] - t[i - 1]);
    before.sort((x, y) => x - y);
    const period = before.length ? before[Math.floor(before.length / 2)] : NaN;
    const window = m.heavy - m.asked;
    const gap = Math.max(0, ...during);
    const control = [];
    for (let i = 1; i < t.length; i++) if (t[i - 1] >= ctrl0 && t[i] <= ctrl1) control.push(t[i] - t[i - 1]);
    const ctrlGap = Math.max(0, ...control);
    console.log(`compile ${Math.round(window)} ms, frame period ${period.toFixed(1)} ms, ${during.length} frames meanwhile, longest gap ${Math.round(gap)} ms`);
    console.log(`control (no compile, ${Math.round(ctrl0)}..${Math.round(ctrl1)}): ${control.length} frames, longest gap ${Math.round(ctrlGap)} ms`);
    // a compile shorter than a few frames cannot show whether frames went on during it: not measured (not passed);
    // the CI lane requires some of its runs to have measured (ASYNC-MEASURED 1)
    if (window < 100 || !(period > 0)) {
      console.log(`not measured on this run: the compile took ${Math.round(window)} ms, under the 100 ms that holds several frames`);
      console.log("ASYNC-MEASURED 0");
    } else if (control.length < Math.ceil(window / 50) - 1) {
      // without a control window holding frames, a stall during the compile cannot be told from the machine's own
      console.log(`not measured on this run: the control window held ${control.length} frames`);
      console.log("ASYNC-MEASURED 0");
    } else {
      // the property is the gap (probe/stall.mjs): a stall is a gap of at least half the window, so frames went on
      // through most of the compile. With no stall the compile held at least two frames
      check("the render thread drew frames while it compiled", during.length >= 2, `${during.length} frames over ${Math.round(window)} ms, ${period.toFixed(1)} ms a frame`);
      // One run cannot tell a compile that blocks the thread from the machine pausing the page (a rare, independent
      // event that seldom hits both windows of one run), so each run says its stalls and the lane counts them over its
      // runs: the compile's stalls may exceed the control window's by at most three (ci.yml derives the margin). The
      // control window is as long as the compile, so both are judged against the same half
      if (isStall(gap, window)) console.log(`STALL compile ${Math.round(gap)} ms`);
      if (isStall(ctrlGap, window)) console.log(`STALL control ${Math.round(ctrlGap)} ms`);
      if (!process.env.PROBE_LANE_COUNTS_STALLS) check("no gap of half the compile or more between the render thread's frames while it compiled", !isStall(gap, window), `${Math.round(gap)} ms over ${during.length} frames while it compiled (${Math.round(window)} ms), ${Math.round(ctrlGap)} ms over ${control.length} frames without`);
      console.log("ASYNC-MEASURED 1");
    }
  }
  // the mirror is the last state the render thread posted: read twice a frame apart, it moves on, and it is a plain read
  // the render thread's beat, not its frames: the mirror moves whether or not the GPU draws
  const a = await page.evaluate(() => { const x = window.__probeMirror(); return { tick: x.tick, sync: typeof x === "object" }; });
  await page.waitForTimeout(200);
  const b = await page.evaluate(() => window.__probeMirror().tick);
  check("the mirror is read at once and follows the render thread", a.sync && b > a.tick, `${a.tick} -> ${b}`);
  const pt = await page.evaluate(() => window.__pageTimes);
  let pg = 0;
  for (let i = 1; i < pt.length; i++) if (pt[i - 1] >= m.asked) pg = Math.max(pg, pt[i] - pt[i - 1]);
  console.log(`longest gap between the page's frames after the slow pipeline was asked: ${Math.round(pg)} ms`);
} else {
  check("without WebGPU the render thread says why", k.includes("fallback"), JSON.stringify(await said()));
  const why = (await said()).find((m) => m.kind === "fallback")?.reason ?? "";
  check("the reason names what was missing", /no WebGPU|no adapter|no device|no webgpu context/.test(why), why);
  // and the page is drawn the other way: WebGL2 in the render thread, on the same canvas (a known colour, read back from the
  // screen)
  for (let i = 0; i < 100 && !(await said()).some((m) => m.kind === "fallback-drawn" || m.kind === "fallback-none"); i++) await page.waitForTimeout(50);
  check("without WebGPU the render thread draws with WebGL2 on the same canvas", (await said()).some((m) => m.kind === "fallback-drawn"), JSON.stringify(await said()));
  await page.waitForTimeout(200);
  const box = await page.locator("canvas").boundingBox();
  const shot = (await page.screenshot({ clip: { x: box.x + box.width / 2, y: box.y + box.height / 2, width: 1, height: 1 } })).toString("base64");
  const px = await page.evaluate(async (b64) => {
    const bm = await createImageBitmap(await (await fetch(`data:image/png;base64,${b64}`)).blob());
    const c = new OffscreenCanvas(1, 1).getContext("2d");
    c.drawImage(bm, 0, 0);
    return Array.from(c.getImageData(0, 0, 1, 1).data.slice(0, 3));
  }, shot);
  // 0.25, 0.5, 0.75 of 255
  const want = [64, 128, 191];
  check("the fallback's colour is on the screen", px.every((v, i) => Math.abs(v - want[i]) <= 2), `${px} (want ${want})`);
}
// what the device said of itself: a loss or an error nobody caught is a failure, with its words
const trouble = (await said()).filter((m) => m.kind === "lost" || m.kind === "uncaptured" || m.kind === "error");
if (skipAsync) {
  // the software adapter loses its device within milliseconds whatever the page does: said, not counted
  console.log(`said, not counted (software adapter): ${JSON.stringify(trouble)}`);
} else {
  check("the device reported no loss and no uncaught error", trouble.length === 0, JSON.stringify(trouble));
}
check("no errors", errors.length === 0, errors.join(" | "));
console.log(JSON.stringify({ compared, problems }));
await browser.close();
if (compared === 0) process.exit(2);
process.exit(problems.length ? 1 : 0);
