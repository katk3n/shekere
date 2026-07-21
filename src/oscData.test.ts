import { describe, expect, it } from "vitest";
import { convertOscSketchData, parseOscMonitorArgs } from "./oscData";

describe("parseOscMonitorArgs", () => {
  it("parses key-value arguments and formats focused values", () => {
    expect(parseOscMonitorArgs(["s", "bd", "gain", 0.756, "n", 3])).toEqual({
      displayText: "s: bd, gain: 0.76, n: 3",
      keyValueArgs: { s: "bd", gain: "0.756", n: "3" },
    });
  });

  it("unwraps object-wrapped OSC arguments", () => {
    expect(parseOscMonitorArgs([{ String: "speed" }, { Float: 1.25 }])).toEqual({
      displayText: "speed: 1.25",
      keyValueArgs: { speed: "1.25" },
    });
  });

  it("shows the first non-focused pair and an ellipsis", () => {
    expect(parseOscMonitorArgs(["foo", 1, "bar", 2])).toEqual({
      displayText: "foo: 1, ...",
      keyValueArgs: { foo: "1", bar: "2" },
    });
  });

  it("limits positional arguments to three values", () => {
    expect(parseOscMonitorArgs([1, 2.345, "three", 4])).toEqual({
      displayText: "1, 2.35, three, ...",
    });
  });

  it("returns an empty display for missing arguments", () => {
    expect(parseOscMonitorArgs(undefined)).toEqual({ displayText: "" });
    expect(parseOscMonitorArgs([])).toEqual({ displayText: "" });
  });
});

describe("convertOscSketchData", () => {
  it("converts even /dirt/play arguments into an object", () => {
    expect(convertOscSketchData("/dirt/play", ["s", "bd", "gain", 0.8])).toEqual({
      s: "bd",
      gain: 0.8,
    });
  });

  it("keeps the last value for duplicate keys", () => {
    expect(convertOscSketchData("/dirt/play", ["s", "bd", "s", "sn"])).toEqual({ s: "sn" });
  });

  it("preserves odd or non-Dirt argument arrays", () => {
    const oddArgs = ["s", "bd", "gain"];
    const otherArgs = ["value", 1];

    expect(convertOscSketchData("/dirt/play", oddArgs)).toBe(oddArgs);
    expect(convertOscSketchData("/other", otherArgs)).toBe(otherArgs);
  });
});
