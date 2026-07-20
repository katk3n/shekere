import { describe, expect, it } from "vitest";
import { applyMidiEvent, createMidiState } from "./midiState";

describe("createMidiState", () => {
  it("creates empty 128-value note and CC arrays", () => {
    const state = createMidiState();

    expect(state.notes).toHaveLength(128);
    expect(state.cc).toHaveLength(128);
    expect(state.notes.every((value) => value === 0)).toBe(true);
    expect(state.cc.every((value) => value === 0)).toBe(true);
  });
});

describe("applyMidiEvent", () => {
  it("normalizes Note On velocity and preserves CC state", () => {
    const state = createMidiState();

    const result = applyMidiEvent(state, { status: 0x92, data1: 60, data2: 64 });

    expect(result.notes[60]).toBeCloseTo(64 / 127);
    expect(result.cc).toBe(state.cc);
    expect(state.notes[60]).toBe(0);
  });

  it("treats zero-velocity Note On as an inactive note", () => {
    const active = applyMidiEvent(createMidiState(), {
      status: 0x90,
      data1: 48,
      data2: 127,
    });

    const result = applyMidiEvent(active, { status: 0x9f, data1: 48, data2: 0 });

    expect(result.notes[48]).toBe(0);
  });

  it("clears notes for Note Off on any MIDI channel", () => {
    const active = applyMidiEvent(createMidiState(), {
      status: 0x90,
      data1: 72,
      data2: 100,
    });

    const result = applyMidiEvent(active, { status: 0x87, data1: 72, data2: 50 });

    expect(result.notes[72]).toBe(0);
  });

  it("normalizes Control Change values and preserves note state", () => {
    const state = createMidiState();

    const result = applyMidiEvent(state, { status: 0xbf, data1: 10, data2: 96 });

    expect(result.cc[10]).toBeCloseTo(96 / 127);
    expect(result.notes).toBe(state.notes);
    expect(state.cc[10]).toBe(0);
  });

  it("clamps incoming values to the MIDI value range", () => {
    const high = applyMidiEvent(createMidiState(), {
      status: 0x90,
      data1: 1,
      data2: 255,
    });
    const low = applyMidiEvent(high, { status: 0xb0, data1: 2, data2: -5 });

    expect(high.notes[1]).toBe(1);
    expect(low.cc[2]).toBe(0);
  });

  it("ignores unsupported MIDI message types", () => {
    const state = createMidiState();

    const result = applyMidiEvent(state, { status: 0xe0, data1: 12, data2: 34 });

    expect(result).toBe(state);
  });

  it("ignores malformed or out-of-range input", () => {
    const state = createMidiState();

    expect(applyMidiEvent(state, { status: 0x90, data1: -1, data2: 10 })).toBe(state);
    expect(applyMidiEvent(state, { status: 0x90, data1: 128, data2: 10 })).toBe(state);
    expect(applyMidiEvent(state, { status: 0x90, data1: 1.5, data2: 10 })).toBe(state);
    expect(applyMidiEvent(state, { status: 0x90, data1: 1, data2: Number.NaN })).toBe(state);
  });
});
