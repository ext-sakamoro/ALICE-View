// What the crate claims, in a real browser: a WebGPU device in a worker on the transferred canvas, a slow pipeline
// compiled while the worker keeps drawing, the page's mirror following the worker's state, and the reason given when
// there is no WebGPU.
//
//   node probe/check.mjs <url of probe/index.html> <webgpu|none>
//
// Author: Moroya Sakamoto
import { chromium } from "playwright";

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
await page.goto(`${url}?steps=${process.env.PROBE_STEPS ?? 96}`);
const said = async () => page.evaluate(() => (window.__probeSaid ? window.__probeSaid() : []));
const kinds = async () => (await said()).map((m) => m.kind);
for (let i = 0; i < 300 && !(await kinds()).some((k) => k === "device" || k === "fallback" || k === "error"); i++) await page.waitForTimeout(100);
const k = await kinds();
if (mode === "webgpu") {
  check("a WebGPU device came in the worker", k.includes("device"), JSON.stringify(await said()));
  // the slow pipeline is asked for at the 10th frame; wait for it
  let m = null;
  for (let i = 0; i < 600; i++) {
    m = await page.evaluate(() => window.__probeMirror());
    if (m && m.heavy !== -1) break;
    await page.waitForTimeout(100);
  }
  check("the slow pipeline was created", m && m.heavy > 0, JSON.stringify(m));
  const t = await page.evaluate(() => window.__probeTimes());
  const during = [];
  for (let i = 1; i < t.length; i++) if (t[i - 1] >= m.asked && t[i] <= m.heavy + 100) during.push(t[i] - t[i - 1]);
  check("the worker drew frames while it compiled", during.length >= 3, `${during.length} frames over ${Math.round(m.heavy - m.asked)} ms`);
  const gap = Math.max(0, ...during);
  console.log(`compile ${Math.round(m.heavy - m.asked)} ms, longest gap between the worker's frames meanwhile ${Math.round(gap)} ms`);
  check("no gap over 50 ms between the worker's frames while it compiled", gap <= 50, `${Math.round(gap)} ms`);
  // the mirror is the last state the worker posted: read twice a frame apart, it moves on, and it is a plain read
  const a = await page.evaluate(() => { const x = window.__probeMirror(); return { frame: x.frame, sync: typeof x === "object" }; });
  await page.waitForTimeout(200);
  const b = await page.evaluate(() => window.__probeMirror().frame);
  check("the mirror is read at once and follows the worker", a.sync && b > a.frame, `${a.frame} -> ${b}`);
  const pt = await page.evaluate(() => window.__pageTimes);
  let pg = 0;
  for (let i = 1; i < pt.length; i++) if (pt[i - 1] >= m.asked) pg = Math.max(pg, pt[i] - pt[i - 1]);
  console.log(`longest gap between the page's frames after the slow pipeline was asked: ${Math.round(pg)} ms`);
} else {
  check("without WebGPU the worker says why", k.includes("fallback"), JSON.stringify(await said()));
  const why = (await said()).find((m) => m.kind === "fallback")?.reason ?? "";
  check("the reason names what was missing", /no WebGPU|no adapter|no device|no webgpu context/.test(why), why);
}
check("no errors", errors.length === 0, errors.join(" | "));
console.log(JSON.stringify({ compared, problems }));
await browser.close();
if (compared === 0) process.exit(2);
process.exit(problems.length ? 1 : 0);
