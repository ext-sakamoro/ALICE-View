// The stall rule on frame gaps of known shape (node --test probe/stall.test.mjs).
import test from "node:test";
import assert from "node:assert/strict";
import { isStall } from "./stall.mjs";

const period = 1000 / 60;

test("a compile that holds the thread the whole time is a stall", () => {
  assert.equal(isStall(259, 259), true);
});

test("a hold of exactly half the compile is a stall, one frame less is not", () => {
  assert.equal(isStall(130, 260), true);
  assert.equal(isStall(130 - period, 260), false);
});

test("a few dropped frames during a long compile are not a stall", () => {
  // measured 2026-11-11: 259 ms compile, longest gap 67 ms (3 frames dropped)
  assert.equal(isStall(67, 259), false);
  assert.equal(isStall(period, 259), false);
});

test("a window with no length measures nothing", () => {
  assert.equal(isStall(0, 0), false);
  assert.equal(isStall(50, 0), false);
});
