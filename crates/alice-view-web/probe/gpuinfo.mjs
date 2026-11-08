// What Chromium uses under the given flags: the WebGL renderer it reports (ANGLE's backend and the Vulkan driver under
// it) and the WebGPU adapter's info: the evidence a lane is named from.
//
//   PROBE_CHROME_ARGS="..." node probe/gpuinfo.mjs <url of a page on a local server (WebGPU needs a secure context)>
//
// Author: Moroya Sakamoto
import { chromium } from "playwright";

const args = ["--enable-unsafe-webgpu", "--enable-features=Vulkan", ...(process.env.PROBE_CHROME_ARGS ?? "").split(" ").filter(Boolean)];
const browser = await chromium.launch({ headless: true, args });
const page = await browser.newPage();
await page.goto(process.argv[2]);
const info = await page.evaluate(async () => {
  const gl = document.createElement("canvas").getContext("webgl2");
  const dbg = gl && gl.getExtension("WEBGL_debug_renderer_info");
  const webgl = dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : gl ? gl.getParameter(gl.RENDERER) : "no WebGL2";
  const a = navigator.gpu ? await navigator.gpu.requestAdapter() : null;
  const webgpu = a ? ["vendor", "architecture", "device", "description"].map((k) => `${k}=${a.info[k] || "-"}`).join(" ") : "no adapter";
  return { webgl, webgpu };
});
console.log(`webgl renderer: ${info.webgl}`);
console.log(`webgpu adapter: ${info.webgpu}`);
if (/no WebGL2/.test(info.webgl) && /no adapter/.test(info.webgpu)) {
  console.log("gpuinfo: read nothing");
  process.exit(2);
}
// PROBE_WEBGL names what the lane is for: the WebGL renderer must contain it (Chromium can fall back to another
// backend without saying so; the lane then is not what its name says)
const wantWebgl = process.env.PROBE_WEBGL;
await browser.close();
if (wantWebgl && !info.webgl.toLowerCase().includes(wantWebgl.toLowerCase())) {
  console.log(`gpuinfo: the WebGL renderer is not ${wantWebgl}`);
  process.exit(1);
}
