export interface PlaylistEntry {
  path: string | null;
  midiNote?: number;
  midiCc?: number;
  oscKey?: string;
  oscValue?: string;
}

export interface Trigger {
  note?: number;
  cc?: number;
}

export interface MidiNavigation {
  next?: Trigger;
  prev?: Trigger;
}

export interface OscTrigger {
  key?: string;
  value?: string;
}

export interface OscNavigation {
  next?: OscTrigger;
  prev?: OscTrigger;
}

export interface MidiInputEvent {
  status: number;
  data1: number;
  data2: number;
}

type NavigationDirection = -1 | 1;

function wrapIndex(index: number, length: number): number {
  return ((index % length) + length) % length;
}

export function findAdjacentPlaylistIndex(
  playlist: readonly PlaylistEntry[],
  currentIndex: number,
  direction: NavigationDirection,
): number {
  if (playlist.length === 0 || playlist.every((entry) => !entry.path)) {
    return currentIndex;
  }

  let target = wrapIndex(currentIndex + direction, playlist.length);
  for (let count = 0; count < playlist.length; count++) {
    if (playlist[target]?.path) return target;
    target = wrapIndex(target + direction, playlist.length);
  }

  return currentIndex;
}

export function resolveKeyboardPlaylistIndex(
  playlist: readonly PlaylistEntry[],
  currentIndex: number,
  key: string,
): number {
  if (key === "ArrowRight") {
    return findAdjacentPlaylistIndex(playlist, currentIndex, 1);
  }
  if (key === "ArrowLeft") {
    return findAdjacentPlaylistIndex(playlist, currentIndex, -1);
  }
  if (key >= "1" && key <= "9") {
    const target = Number.parseInt(key, 10) - 1;
    if (target < playlist.length && playlist[target]?.path) return target;
  }

  return currentIndex;
}

export function resolveMidiPlaylistIndex(
  playlist: readonly PlaylistEntry[],
  currentIndex: number,
  navigation: MidiNavigation,
  event: MidiInputEvent,
): number {
  const { status, data1, data2 } = event;
  const type = status & 0xf0;
  let target = currentIndex;

  playlist.forEach((entry, index) => {
    if (!entry.path) return;
    if (type === 0x90 && entry.midiNote !== undefined && data1 === entry.midiNote && data2 > 0) {
      target = index;
    }
    if (type === 0xb0 && entry.midiCc !== undefined && data1 === entry.midiCc) {
      target = index;
    }
  });

  if (type === 0x90 && data2 > 0) {
    if (data1 === navigation.next?.note) {
      target = findAdjacentPlaylistIndex(playlist, currentIndex, 1);
    }
    if (data1 === navigation.prev?.note) {
      target = findAdjacentPlaylistIndex(playlist, currentIndex, -1);
    }
  }
  if (type === 0xb0) {
    if (data1 === navigation.next?.cc) {
      target = findAdjacentPlaylistIndex(playlist, currentIndex, 1);
    }
    if (data1 === navigation.prev?.cc) {
      target = findAdjacentPlaylistIndex(playlist, currentIndex, -1);
    }
  }

  return target;
}

export function resolveOscPlaylistIndex(
  playlist: readonly PlaylistEntry[],
  currentIndex: number,
  navigation: OscNavigation,
  args: Readonly<Record<string, string>> | undefined,
): number {
  if (!args) return currentIndex;

  let target = currentIndex;
  playlist.forEach((entry, index) => {
    if (!entry.path) return;
    if (entry.oscKey && entry.oscValue && args[entry.oscKey] === entry.oscValue) {
      target = index;
    }
  });

  const matches = (trigger?: OscTrigger): boolean => (
    Boolean(trigger?.key && trigger.value && args[trigger.key] === trigger.value)
  );

  if (matches(navigation.next)) {
    target = findAdjacentPlaylistIndex(playlist, currentIndex, 1);
  }
  if (matches(navigation.prev)) {
    target = findAdjacentPlaylistIndex(playlist, currentIndex, -1);
  }

  return target;
}
