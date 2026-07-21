export interface MidiState {
  notes: number[];
  cc: number[];
}

export interface MidiEvent {
  status: number;
  data1: number;
  data2: number;
}

const MIDI_VALUE_MAX = 127;

export function createMidiState(): MidiState {
  return {
    notes: new Array<number>(128).fill(0),
    cc: new Array<number>(128).fill(0),
  };
}

function isValidMidiEvent(event: MidiEvent): boolean {
  return (
    Number.isInteger(event.status) &&
    Number.isInteger(event.data1) &&
    Number.isFinite(event.data2) &&
    event.data1 >= 0 &&
    event.data1 <= MIDI_VALUE_MAX
  );
}

function normalizeMidiValue(value: number): number {
  return Math.min(MIDI_VALUE_MAX, Math.max(0, value)) / MIDI_VALUE_MAX;
}

export function applyMidiEvent(state: MidiState, event: MidiEvent): MidiState {
  if (!isValidMidiEvent(event)) return state;

  const type = event.status & 0xf0;
  if (type === 0x90) {
    const notes = [...state.notes];
    notes[event.data1] = normalizeMidiValue(event.data2);
    return { ...state, notes };
  }
  if (type === 0x80) {
    const notes = [...state.notes];
    notes[event.data1] = 0;
    return { ...state, notes };
  }
  if (type === 0xb0) {
    const cc = [...state.cc];
    cc[event.data1] = normalizeMidiValue(event.data2);
    return { ...state, cc };
  }

  return state;
}
