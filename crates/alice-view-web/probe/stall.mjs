// When a frame gap counts as a stall (probe/check.mjs, tested by probe/stall.test.mjs).
//
// A compile that blocks the render thread leaves no frame for about as long as it runs, so its longest gap is close to
// the whole compile. A compile that only competes for the CPU (a software adapter compiling on the same cores) drops a
// frame or a few, and the gap stays a small part of the compile. A stall is therefore a gap of at least half the window
// it is measured in: it tells a blocking compile from contention on a fast machine and on a slow one alike, which a
// fixed number of milliseconds does not (2026-11-11: a 50 ms threshold counted 2-3 dropped frames during a 259 ms
// compile as stalls on one runner and not on another).
export function isStall(gap, window) {
  return window > 0 && gap >= window / 2;
}
