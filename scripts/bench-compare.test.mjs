import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { compare, isRegression, readBaseline, report } from "./bench-compare.mjs";

const estimate = (mean, spread = 1) => ({ mean, lower: mean - spread, upper: mean + spread });

test("over 10% slower with separate intervals is a regression", () => {
  assert.equal(isRegression(estimate(100), estimate(120)), true);
});

test("exactly 10% slower is not", () => {
  assert.equal(isRegression(estimate(100), estimate(110)), false);
});

test("a large change inside the noise is not", () => {
  assert.equal(isRegression(estimate(100, 20), estimate(120, 20)), false);
});

test("getting faster is not", () => {
  assert.equal(isRegression(estimate(100), estimate(50)), false);
});

test("new benchmarks are reported but cannot regress", () => {
  const rows = compare(new Map(), new Map([["a", estimate(100)]]));
  assert.equal(rows[0].regression, false);
  assert.match(report(rows), /\| a \|  \| 100\.0 ns \| new \|/);
});

test("removed benchmarks are ignored", () => {
  assert.deepEqual(compare(new Map([["gone", estimate(100)]]), new Map()), []);
});

test("reads Criterion's layout and keeps each benchmark's fastest round", () => {
  const home = mkdtempSync(join(tmpdir(), "bench-compare-"));
  const write = (baseline, mean) => {
    const dir = join(home, "store", "save_1000_clips", baseline);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "benchmark.json"), JSON.stringify({ full_id: "store/save_1000_clips" }));
    writeFileSync(
      join(dir, "estimates.json"),
      JSON.stringify({
        mean: {
          point_estimate: mean,
          confidence_interval: { lower_bound: mean - 1, upper_bound: mean + 1 },
        },
      }),
    );
  };
  try {
    write("head-1", 9);
    write("head-2", 5);
    write("head-3", 7);
    write("new", 1); // Criterion's own latest run; not a round.
    assert.deepEqual(
      [...readBaseline(home, "head")],
      [["store/save_1000_clips", { mean: 5, lower: 4, upper: 6 }]],
    );
    assert.equal(readBaseline(home, "base").size, 0);
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
});
