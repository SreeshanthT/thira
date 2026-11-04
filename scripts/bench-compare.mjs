// Fails a pull request when a benchmark got over 10% slower (issue #18).
//
// Reads results that Criterion saved in one CRITERION_HOME as baselines named
// <base>-<round> and <head>-<round>, measured alternately on one machine. Results
// from different runs or machines are not comparable, and even identical code
// measured twice in a row can differ by over 30% when other work interferes.
// Interference only ever slows code down, so each side is represented by its
// fastest round. A slowdown also needs non-overlapping confidence intervals.
//
// Usage: node scripts/bench-compare.mjs <criterion home> <base name> <head name>

import { appendFileSync, existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const THRESHOLD = 1.1;

/** Compares `{ mean, lower, upper }` estimates in nanoseconds. */
export function isRegression(base, head) {
  return head.mean / base.mean > THRESHOLD && head.lower > base.upper;
}

/**
 * Finds `<id>/<baseline>-<round>/estimates.json` below `dir`; returns a Map of
 * benchmark ID to the estimates of its fastest round.
 */
export function readBaseline(dir, baseline) {
  const round = new RegExp(`^${baseline}-\\d+$`);
  const results = new Map();
  const visit = (current) => {
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      const path = join(current, entry.name);
      const children = readdirSync(path, { withFileTypes: true }).filter((c) => c.isDirectory());
      const rounds = children.filter(
        (c) => round.test(c.name) && existsSync(join(path, c.name, "estimates.json")),
      );
      for (const { name } of rounds) {
        const { full_id } = JSON.parse(readFileSync(join(path, name, "benchmark.json"), "utf8"));
        const { point_estimate, confidence_interval } = JSON.parse(
          readFileSync(join(path, name, "estimates.json"), "utf8"),
        ).mean;
        const best = results.get(full_id);
        if (!best || point_estimate < best.mean) {
          results.set(full_id, {
            mean: point_estimate,
            lower: confidence_interval.lower_bound,
            upper: confidence_interval.upper_bound,
          });
        }
      }
      // A directory of rounds is a benchmark; anything else may contain some.
      if (children.every((c) => !existsSync(join(path, c.name, "estimates.json")))) visit(path);
    }
  };
  if (existsSync(dir)) visit(dir);
  return results;
}

/** One row per benchmark in `head`. `base` is absent for new benchmarks. */
export function compare(baseResults, headResults) {
  return [...headResults]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([id, head]) => {
      const base = baseResults.get(id);
      return {
        id,
        base,
        head,
        change: base ? head.mean / base.mean - 1 : undefined,
        regression: base ? isRegression(base, head) : false,
      };
    });
}

function duration(ns) {
  const units = [
    [1e9, "s"],
    [1e6, "ms"],
    [1e3, "µs"],
  ];
  for (const [size, unit] of units) if (ns >= size) return `${(ns / size).toFixed(2)} ${unit}`;
  return `${ns.toFixed(1)} ns`;
}

export function report(rows) {
  const lines = ["| Benchmark | Base | Head | Change | |", "| --- | ---: | ---: | ---: | --- |"];
  for (const { id, base, head, change, regression } of rows) {
    const percent = change === undefined ? "new" : `${change >= 0 ? "+" : ""}${(change * 100).toFixed(1)}%`;
    const mark = regression ? "Regression" : "";
    lines.push(`| ${id} | ${base ? duration(base.mean) : ""} | ${duration(head.mean)} | ${percent} | ${mark} |`);
  }
  return lines.join("\n");
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [home, baseName, headName] = process.argv.slice(2);
  const rows = compare(readBaseline(home, baseName), readBaseline(home, headName));
  if (rows.length === 0) {
    console.error(`No "${headName}" benchmark results in ${home}.`);
    process.exit(1);
  }
  const table = report(rows);
  console.log(table);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${table}\n`);
  const slower = rows.filter((row) => row.regression);
  if (slower.length > 0) {
    console.error(`\n${slower.length} benchmark(s) got over 10% slower: ${slower.map((r) => r.id).join(", ")}`);
    process.exit(1);
  }
}
