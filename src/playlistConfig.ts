import { parse as parseToml } from "smol-toml";
import type {
  MidiNavigation,
  OscNavigation,
  PlaylistEntry,
} from "./playlistNavigation";

interface PlaylistToml {
  sketch?: Array<{
    file: string;
    midi_note?: number;
    midi_cc?: number;
    osc_key?: string;
    osc_value?: string;
  }>;
  midi?: {
    navigation?: MidiNavigation;
  };
  osc?: {
    navigation?: OscNavigation;
  };
}

export interface ParsedPlaylistConfig {
  playlist?: PlaylistEntry[];
  midiNavigation?: MidiNavigation;
  oscNavigation?: OscNavigation;
}

function getBaseDirectory(playlistPath: string): string {
  const posixDirectory = playlistPath.substring(0, playlistPath.lastIndexOf("/") + 1);
  return posixDirectory || playlistPath.substring(0, playlistPath.lastIndexOf("\\") + 1);
}

function resolveSketchPath(baseDirectory: string, sketchPath: string): string {
  return sketchPath.startsWith("/") || sketchPath.includes(":")
    ? sketchPath
    : baseDirectory + sketchPath;
}

export function parsePlaylistConfig(
  content: string,
  playlistPath: string,
): ParsedPlaylistConfig {
  const data = parseToml(content) as unknown as PlaylistToml;
  let playlist: PlaylistEntry[] | undefined;

  if (data.sketch && Array.isArray(data.sketch)) {
    const baseDirectory = getBaseDirectory(playlistPath);
    playlist = data.sketch.map((sketch) => ({
      path: resolveSketchPath(baseDirectory, sketch.file),
      midiNote: sketch.midi_note,
      midiCc: sketch.midi_cc,
      oscKey: sketch.osc_key,
      oscValue: sketch.osc_value,
    }));

    while (playlist.length < 9) playlist.push({ path: null });
  }

  return {
    playlist,
    midiNavigation: data.midi?.navigation,
    oscNavigation: data.osc?.navigation,
  };
}
