import { describe, expect, it } from "vitest";
import { parsePlaylistConfig } from "./playlistConfig";

describe("parsePlaylistConfig", () => {
  it("resolves POSIX relative paths and preserves absolute paths", () => {
    const result = parsePlaylistConfig(`
      [[sketch]]
      file = "relative.js"

      [[sketch]]
      file = "/absolute/sketch.js"

      [[sketch]]
      file = "C:\\\\sketches\\\\windows.js"
    `, "/shows/current/playlist.toml");

    expect(result.playlist?.slice(0, 3).map((entry) => entry.path)).toEqual([
      "/shows/current/relative.js",
      "/absolute/sketch.js",
      "C:\\sketches\\windows.js",
    ]);
  });

  it("resolves relative paths beside a Windows playlist", () => {
    const result = parsePlaylistConfig(`
      [[sketch]]
      file = "relative.js"
    `, "C:\\shows\\current\\playlist.toml");

    expect(result.playlist?.[0].path).toBe("C:\\shows\\current\\relative.js");
  });

  it("maps sketch metadata and navigation settings", () => {
    const result = parsePlaylistConfig(`
      [midi.navigation.next]
      note = 60

      [midi.navigation.prev]
      cc = 10

      [osc.navigation.next]
      key = "scene"
      value = "next"

      [osc.navigation.prev]
      key = "scene"
      value = "prev"

      [[sketch]]
      file = "first.js"
      midi_note = 48
      midi_cc = 12
      osc_key = "s"
      osc_value = "bd"
    `, "/shows/playlist.toml");

    expect(result.playlist?.[0]).toEqual({
      path: "/shows/first.js",
      midiNote: 48,
      midiCc: 12,
      oscKey: "s",
      oscValue: "bd",
    });
    expect(result.midiNavigation).toEqual({ next: { note: 60 }, prev: { cc: 10 } });
    expect(result.oscNavigation).toEqual({
      next: { key: "scene", value: "next" },
      prev: { key: "scene", value: "prev" },
    });
  });

  it("pads playlists to nine slots", () => {
    const result = parsePlaylistConfig(`
      [[sketch]]
      file = "first.js"
    `, "/shows/playlist.toml");

    expect(result.playlist).toHaveLength(9);
    expect(result.playlist?.slice(1)).toEqual(
      new Array(8).fill(null).map(() => ({ path: null })),
    );
  });

  it("does not truncate playlists longer than nine slots", () => {
    const sketches = Array.from(
      { length: 10 },
      (_, index) => `[[sketch]]\nfile = "${index}.js"`,
    ).join("\n");

    const result = parsePlaylistConfig(sketches, "/shows/playlist.toml");

    expect(result.playlist).toHaveLength(10);
    expect(result.playlist?.[9].path).toBe("/shows/9.js");
  });

  it("leaves the playlist unset when the TOML has only navigation", () => {
    const result = parsePlaylistConfig(`
      [midi.navigation.next]
      cc = 21
    `, "/shows/playlist.toml");

    expect(result.playlist).toBeUndefined();
    expect(result.midiNavigation).toEqual({ next: { cc: 21 } });
    expect(result.oscNavigation).toBeUndefined();
  });

  it("propagates malformed TOML errors", () => {
    expect(() => parsePlaylistConfig("[[sketch]", "/shows/playlist.toml")).toThrow();
  });
});
