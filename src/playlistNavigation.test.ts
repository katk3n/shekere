import { describe, expect, it } from "vitest";
import {
  findAdjacentPlaylistIndex,
  resolveKeyboardPlaylistIndex,
  resolveMidiPlaylistIndex,
  resolveOscPlaylistIndex,
  type PlaylistEntry,
} from "./playlistNavigation";

const entry = (path: string | null, metadata: Partial<PlaylistEntry> = {}): PlaylistEntry => ({
  path,
  ...metadata,
});

describe("findAdjacentPlaylistIndex", () => {
  it("moves in either direction and skips empty slots", () => {
    const playlist = [entry("a.js"), entry(null), entry(null), entry("d.js")];

    expect(findAdjacentPlaylistIndex(playlist, 0, 1)).toBe(3);
    expect(findAdjacentPlaylistIndex(playlist, 3, -1)).toBe(0);
  });

  it("wraps at both ends", () => {
    const playlist = [entry("a.js"), entry(null), entry("c.js")];

    expect(findAdjacentPlaylistIndex(playlist, 2, 1)).toBe(0);
    expect(findAdjacentPlaylistIndex(playlist, 0, -1)).toBe(2);
  });

  it("keeps the current index when no playable slot exists", () => {
    expect(findAdjacentPlaylistIndex([], 0, 1)).toBe(0);
    expect(findAdjacentPlaylistIndex([entry(null), entry(null)], 1, -1)).toBe(1);
  });
});

describe("resolveKeyboardPlaylistIndex", () => {
  const playlist = [entry("a.js"), entry(null), entry("c.js")];

  it("handles arrow navigation", () => {
    expect(resolveKeyboardPlaylistIndex(playlist, 0, "ArrowRight")).toBe(2);
    expect(resolveKeyboardPlaylistIndex(playlist, 0, "ArrowLeft")).toBe(2);
  });

  it("selects populated slots with number keys", () => {
    expect(resolveKeyboardPlaylistIndex(playlist, 0, "3")).toBe(2);
  });

  it("ignores empty, out-of-range, and unrelated keys", () => {
    expect(resolveKeyboardPlaylistIndex(playlist, 0, "2")).toBe(0);
    expect(resolveKeyboardPlaylistIndex(playlist, 0, "9")).toBe(0);
    expect(resolveKeyboardPlaylistIndex(playlist, 0, "Enter")).toBe(0);
  });
});

describe("resolveMidiPlaylistIndex", () => {
  const playlist = [
    entry("a.js"),
    entry(null),
    entry("c.js", { midiNote: 60, midiCc: 12 }),
    entry("d.js", { midiNote: 60, midiCc: 12 }),
  ];

  it("uses the status high nibble and selects the last matching Note On slot", () => {
    expect(resolveMidiPlaylistIndex(playlist, 0, {}, { status: 0x92, data1: 60, data2: 127 })).toBe(3);
  });

  it("ignores Note On with zero velocity and Note Off", () => {
    expect(resolveMidiPlaylistIndex(playlist, 0, {}, { status: 0x90, data1: 60, data2: 0 })).toBe(0);
    expect(resolveMidiPlaylistIndex(playlist, 0, {}, { status: 0x80, data1: 60, data2: 127 })).toBe(0);
  });

  it("selects the last matching Control Change slot", () => {
    expect(resolveMidiPlaylistIndex(playlist, 0, {}, { status: 0xb7, data1: 12, data2: 0 })).toBe(3);
  });

  it("applies navigation after direct slot matching", () => {
    expect(resolveMidiPlaylistIndex(
      playlist,
      0,
      { next: { note: 60 } },
      { status: 0x90, data1: 60, data2: 100 },
    )).toBe(2);
  });

  it("preserves prev precedence when next and prev use the same trigger", () => {
    expect(resolveMidiPlaylistIndex(
      playlist,
      2,
      { next: { cc: 12 }, prev: { cc: 12 } },
      { status: 0xb0, data1: 12, data2: 127 },
    )).toBe(0);
  });
});

describe("resolveOscPlaylistIndex", () => {
  const playlist = [
    entry("a.js"),
    entry(null),
    entry("c.js", { oscKey: "scene", oscValue: "chorus" }),
    entry("d.js", { oscKey: "scene", oscValue: "chorus" }),
  ];

  it("selects the last slot whose key and value match", () => {
    expect(resolveOscPlaylistIndex(playlist, 0, {}, { scene: "chorus" })).toBe(3);
  });

  it("keeps the current slot for missing or non-matching arguments", () => {
    expect(resolveOscPlaylistIndex(playlist, 0, {}, undefined)).toBe(0);
    expect(resolveOscPlaylistIndex(playlist, 0, {}, { scene: "verse" })).toBe(0);
  });

  it("applies navigation after direct slot matching", () => {
    expect(resolveOscPlaylistIndex(
      playlist,
      0,
      { next: { key: "scene", value: "chorus" } },
      { scene: "chorus" },
    )).toBe(2);
  });

  it("requires non-empty navigation keys and values", () => {
    expect(resolveOscPlaylistIndex(
      playlist,
      0,
      { next: { key: "", value: "chorus" } },
      { "": "chorus" },
    )).toBe(0);
  });

  it("preserves prev precedence when next and prev use the same trigger", () => {
    expect(resolveOscPlaylistIndex(
      playlist,
      2,
      {
        next: { key: "direction", value: "go" },
        prev: { key: "direction", value: "go" },
      },
      { direction: "go" },
    )).toBe(0);
  });
});
