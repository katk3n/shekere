import { useState, useEffect, useRef } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { readTextFile, watch } from "@tauri-apps/plugin-fs";
import { emit, listen } from "@tauri-apps/api/event";
import { 
  FileCode, 
  AlertCircle, 
  Mic, 
  MicOff, 
  Sparkles, 
  Activity, 
  Radio, 
  Music, 
  Volume2, 
  ListMusic, 
  ChevronRight, 
  ChevronLeft,
  Settings,
  Eye
} from "lucide-react";
import { useAudioAnalyzer } from "./hooks/useAudioAnalyzer";
import { parse as parseToml } from "smol-toml";
import shekereIcon from "./assets/shekere-icon.png";

// --- Types ---
interface PlaylistEntry {
  path: string | null;
  midiNote?: number;
  midiCc?: number;
}

interface MidiNavigation {
  next_note?: number;
  prev_note?: number;
  next_cc?: number;
  prev_cc?: number;
}

interface PlaylistToml {
  sketch?: Array<{
    file: string;
    midi_note?: number;
    midi_cc?: number;
  }>;
  midi?: {
    navigation?: MidiNavigation;
  };
}

// --- Helper Components ---
const LevelBar = ({ label, value, colorClass }: { label: string, value: number, colorClass: string }) => (
  <div className="flex flex-col gap-1 w-full">
    <div className="flex justify-between items-center text-[10px] sm:text-xs font-semibold text-neutral-500 uppercase tracking-wider">
      <span>{label}</span>
      <span className="text-neutral-400 font-mono">{(value * 100).toFixed(0)}%</span>
    </div>
    <div className="h-2 w-full bg-neutral-700/50 rounded-full overflow-hidden">
      <div
        className={`h-full ${colorClass} transition-all duration-[50ms] ease-out min-w-[2%]`}
        style={{ width: `${Math.min(Math.max(value * 100, 0), 100)}%` }}
      />
    </div>
  </div>
);

const Indicator = ({ label, icon: Icon, active, text, subText }: { label: string, icon: any, active: boolean, text: string, subText?: string }) => (
  <div className="flex items-center gap-3 bg-neutral-800 p-3 rounded-lg border border-neutral-700/50 shadow-sm">
    <div className="relative flex items-center justify-center shrink-0">
      <Icon className="w-5 h-5 text-neutral-500" />
      <div className={`absolute -top-1 -right-1 w-2.5 h-2.5 rounded-full transition-colors duration-[50ms] ${active ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.8)]' : 'bg-neutral-700'}`} />
    </div>
    <div className="flex flex-col flex-1 min-w-0 justify-center">
      <div className="flex items-baseline gap-2 mb-0.5">
        <span className="text-[10px] text-neutral-400 uppercase tracking-widest font-semibold shrink-0">{label}</span>
        <span className="text-sm font-mono text-neutral-200 truncate" title={text}>{text}</span>
      </div>
      {(subText || subText === "") && (
        <span className="text-xs font-mono text-neutral-400 truncate" title={subText || "No parameters"}>
          {subText || "No metadata"}
        </span>
      )}
    </div>
  </div>
);

export default function App() {
  const [playlist, setPlaylist] = useState<PlaylistEntry[]>(
    Array(9).fill(null).map(() => ({ path: null }))
  );
  const [currentIndex, setCurrentIndex] = useState(0);
  const [midiNavigation, setMidiNavigation] = useState<MidiNavigation>({});
  const [error, setError] = useState<string | null>(null);
  
  const { isActive: isAudioActive, start: startAudio, stop: stopAudio, error: audioError } = useAudioAnalyzer();

  // FX Settings
  const [bloomStrength, setBloomStrength] = useState(0);
  const [bloomRadius, setBloomRadius] = useState(0);
  const [bloomThreshold, setBloomThreshold] = useState(1.0);
  const [rgbShiftAmount, setRgbShiftAmount] = useState(0);
  const [filmIntensity, setFilmIntensity] = useState(0);
  const [vignetteOffset, setVignetteOffset] = useState(0);
  const [vignetteDarkness, setVignetteDarkness] = useState(1.0);

  const [activeFxTab, setActiveFxTab] = useState<'bloom' | 'rgbShift' | 'film' | 'vignette'>('bloom');

  // Signal Activity
  const [audioLevels, setAudioLevels] = useState({ volume: 0, bass: 0, mid: 0, high: 0 });
  const [lastMidi, setLastMidi] = useState<{ text: string, subText: string, id: number, status: number, data1: number, data2: number } | null>(null);
  const [lastOsc, setLastOsc] = useState<{ text: string, subText: string, id: number } | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  const currentSketch = playlist[currentIndex]?.path;

  // --- Switching Logic ---
  const switchIndex = (newIndex: number) => {
    if (playlist.every(p => !p.path)) return; // All empty

    const direction = newIndex > currentIndex ? 1 : -1;
    let target = newIndex;
    
    // Safety wrap
    if (target < 0) target = playlist.length - 1;
    if (target >= playlist.length) target = 0;

    // Search for next non-empty
    let count = 0;
    while (!playlist[target]?.path && count < playlist.length) {
      target += direction;
      if (target < 0) target = playlist.length - 1;
      if (target >= playlist.length) target = 0;
      count++;
    }

    if (playlist[target]?.path) {
      setCurrentIndex(target);
    }
  };

  // --- Keyboard Triggers ---
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "ArrowRight") switchIndex(currentIndex + 1);
      if (e.key === "ArrowLeft") switchIndex(currentIndex - 1);
      if (e.key >= "1" && e.key <= "9") {
        const idx = parseInt(e.key) - 1;
        if (idx < playlist.length && playlist[idx].path) setCurrentIndex(idx);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [currentIndex, playlist.length]);

  // --- MIDI Triggers ---
  useEffect(() => {
    if (!lastMidi) return;
    const { status, data1, data2 } = lastMidi;
    const type = status & 0xF0;

    // Direct Jump
    playlist.forEach((entry, idx) => {
      if (!entry.path) return;
      if (type === 0x90 && entry.midiNote !== undefined && data1 === entry.midiNote && data2 > 0) {
        setCurrentIndex(idx);
      }
      if (type === 0xB0 && entry.midiCc !== undefined && data1 === entry.midiCc) {
        setCurrentIndex(idx);
      }
    });

    // Navigation
    if (type === 0x90 && data2 > 0) {
      if (data1 === midiNavigation.next_note) switchIndex(currentIndex + 1);
      if (data1 === midiNavigation.prev_note) switchIndex(currentIndex - 1);
    }
    if (type === 0xB0) {
      if (data1 === midiNavigation.next_cc) switchIndex(currentIndex + 1);
      if (data1 === midiNavigation.prev_cc) switchIndex(currentIndex - 1);
    }
  }, [lastMidi]);

  // --- Signal Listeners ---
  useEffect(() => {
    const u1 = listen<any>('audio-activity', (e) => {
      setAudioLevels(e.payload);
      if (canvasRef.current && e.payload.bands && e.payload.bands.length > 0) {
        const canvas = canvasRef.current;
        const ctx = canvas.getContext('2d');
        if (ctx) {
          const w = canvas.width;
          const h = canvas.height;
          ctx.clearRect(0, 0, w, h);
          ctx.fillStyle = '#10b981';
          const bands = e.payload.bands;
          const barWidth = w / bands.length;
          for (let i = 0; i < bands.length; i++) {
            const barHeight = bands[i] * h;
            ctx.fillRect(i * barWidth, h - barHeight, Math.max(1, barWidth - 0.5), barHeight);
          }
        }
      }
    });

    const u2 = listen<any>('midi-event', (e) => {
      const p = e.payload;
      const t = p.status & 0xF0;
      const typeStr = t === 0x90 ? 'Note On' : t === 0x80 ? 'Note Off' : t === 0xB0 ? 'Control Change' : 'Signal';
      setLastMidi({ 
        text: typeStr, 
        subText: `data: [${p.data1}], val: ${p.data2}`, 
        id: Date.now(),
        status: p.status,
        data1: p.data1,
        data2: p.data2
      });
    });

    const u3 = listen<any>('osc-event', (e) => {
      const p = e.payload;
      let argsStr = "";
      if (p.args && Array.isArray(p.args)) {
        const rawArgs = p.args.map((a: any) => {
          if (typeof a === 'object' && a !== null) {
            const vals = Object.values(a);
            return vals.length > 0 ? vals[0] : JSON.stringify(a);
          }
          return a;
        });
        let isKvFormat = rawArgs.length >= 2 && rawArgs.length % 2 === 0;
        for (let i = 0; i < rawArgs.length; i += 2) {
          if (typeof rawArgs[i] !== 'string') { isKvFormat = false; break; }
        }
        if (isKvFormat) {
          const focusKeys = ['s', 'n', 'cps', 'note', 'gain', 'speed', 'vowel'];
          const pairs: string[] = [];
          for (let i = 0; i < rawArgs.length; i += 2) {
            const key = String(rawArgs[i]);
            const val = rawArgs[i + 1];
            if (focusKeys.includes(key)) {
              const fmtVal = typeof val === 'number' ? (Number.isInteger(val) ? val.toString() : val.toFixed(2)) : String(val);
              pairs.push(`${key}: ${fmtVal}`);
            }
          }
          if (pairs.length === 0) {
            pairs.push(`${rawArgs[0]}: ${rawArgs[1]}`);
            if (rawArgs.length > 2) pairs.push('...');
          }
          argsStr = pairs.join(', ');
        } else {
          const limitedArgs = rawArgs.slice(0, 3).map((a: any) => typeof a === 'number' ? (Number.isInteger(a) ? a.toString() : a.toFixed(2)) : String(a));
          argsStr = limitedArgs.join(', ');
          if (rawArgs.length > 3) argsStr += ', ...';
        }
      }
      setLastOsc({ text: p.address, subText: argsStr, id: Date.now() });
    });

    const u4 = listen<any>('preview-frame', (e) => {
      setPreviewUrl(e.payload.dataUrl);
    });

    return () => {
      u1.then(f => f()); u2.then(f => f()); u3.then(f => f()); u4.then(f => f());
    };
  }, []);

  // --- FX Sync ---
  const skipNextEmitRef = useRef(false);

  useEffect(() => {
    if (skipNextEmitRef.current) {
      skipNextEmitRef.current = false;
      return;
    }
    emit("update-fx-settings", {
      bloom: { strength: bloomStrength, radius: bloomRadius, threshold: bloomThreshold },
      rgbShift: { amount: rgbShiftAmount },
      film: { intensity: filmIntensity },
      vignette: { offset: vignetteOffset, darkness: vignetteDarkness }
    });
  }, [bloomStrength, bloomRadius, bloomThreshold, rgbShiftAmount, filmIntensity, vignetteOffset, vignetteDarkness]);

  useEffect(() => {
    const unlistenPromise = listen<any>(
      "fx-settings-changed",
      (event) => {
        const { bloom, rgbShift, film, vignette } = event.payload;
        skipNextEmitRef.current = true;
        if (bloom) {
          setBloomStrength(bloom.strength);
          setBloomRadius(bloom.radius);
          setBloomThreshold(bloom.threshold);
        }
        if (rgbShift && rgbShift.amount !== undefined) setRgbShiftAmount(rgbShift.amount);
        if (film && film.intensity !== undefined) setFilmIntensity(film.intensity);
        if (vignette) {
          if (vignette.offset !== undefined) setVignetteOffset(vignette.offset);
          if (vignette.darkness !== undefined) setVignetteDarkness(vignette.darkness);
        }
      }
    );
    return () => { unlistenPromise.then((unlisten) => unlisten()); };
  }, []);

  // --- Code Loading & Watching ---
  useEffect(() => {
    if (!currentSketch) return;

    let unwatch: (() => void) | null = null;
    let lastEmitTime = 0;
    const THROTTLE_MS = 150;

    const loadAndEmit = async () => {
      try {
        const code = await readTextFile(currentSketch);
        await emit("user-code-update", { code });
        setError(null);
      } catch (err: any) {
        console.error("Failed to read or emit file:", err);
        setError(`Failed to read file: ${err.message || err}`);
      }
    };

    loadAndEmit();

    watch(
      currentSketch,
      (event) => {
        if (
          event.type === "any" ||
          event.type === "other" ||
          (typeof event.type === "object" && "modify" in event.type)
        ) {
          const now = Date.now();
          if (now - lastEmitTime > THROTTLE_MS) {
            lastEmitTime = now;
            loadAndEmit();
          }
        }
      },
      { recursive: false, delayMs: 20 }
    ).then((unwatchFn) => {
      unwatch = unwatchFn;
    }).catch((err: any) => {
      console.error("Failed to start watcher:", err);
      setError(`Failed to start watcher: ${err.message || err}`);
    });

    return () => { if (unwatch) unwatch(); };
  }, [currentSketch, currentIndex]);

  // --- File Handlers ---
  const handleSelectSlot = async (index: number) => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: "JavaScript", extensions: ["js"] }],
      });
      if (selected && typeof selected === "string") {
        const newPlaylist = [...playlist];
        newPlaylist[index] = { ...newPlaylist[index], path: selected };
        setPlaylist(newPlaylist);
        if (index === currentIndex) {
          setCurrentIndex(-1);
          setTimeout(() => setCurrentIndex(index), 10);
        }
      }
    } catch (err: any) {
      setError(`Dialog failed: ${err.message || err}`);
    }
  };

  const handleLoadPlaylist = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: "TOML Playlist", extensions: ["toml"] }],
      });
      if (selected && typeof selected === "string") {
        const content = await readTextFile(selected);
        const data = parseToml(content) as unknown as PlaylistToml;
        
        const baseDir = selected.substring(0, selected.lastIndexOf('/') + 1) || selected.substring(0, selected.lastIndexOf('\\') + 1);

        if (data.sketch && Array.isArray(data.sketch)) {
          const newPlaylist: PlaylistEntry[] = data.sketch.map((s: any) => ({
            path: s.file.startsWith('/') || s.file.includes(':') ? s.file : baseDir + s.file,
            midiNote: s.midi_note,
            midiCc: s.midi_cc
          }));

          while (newPlaylist.length < 9) newPlaylist.push({ path: null });
          setPlaylist(newPlaylist);
          setCurrentIndex(0);
        }

        if (data.midi && data.midi.navigation) {
          setMidiNavigation(data.midi.navigation);
        }
        setError(null);
      }
    } catch (err: any) {
      setError(`Failed to load playlist: ${err.message}`);
    }
  };

  const isMidiActive = lastMidi && (Date.now() - lastMidi.id < 150);
  const isOscActive = lastOsc && (Date.now() - lastOsc.id < 150);

  return (
    <div className="min-h-screen bg-neutral-900 text-neutral-100 font-sans flex flex-col">
      <div className="w-full flex-1 mx-auto">
        <div className="p-4 border-b border-neutral-800 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <img src={shekereIcon} alt="Shekere" className="w-8 h-8 object-contain mix-blend-screen drop-shadow-[0_0_8px_rgba(59,130,246,0.5)]" />
            <h1 className="text-xl font-bold tracking-tight">Shekere</h1>
          </div>
          <div className="flex gap-2">
             <button
                onClick={handleLoadPlaylist}
                className="flex items-center gap-2 bg-neutral-800 hover:bg-neutral-700 active:bg-neutral-600 text-neutral-200 px-3 py-1.5 rounded-lg text-xs font-semibold border border-neutral-700 transition-all shadow-sm"
              >
                <Settings className="w-3.5 h-3.5" />
                Load Playlist
              </button>
             <button
                onClick={() => { if (isAudioActive) stopAudio(); else startAudio(); }}
                className={`flex items-center gap-2 ${isAudioActive ? "bg-red-500/20 text-red-400 border-red-500/50" : "bg-emerald-500/20 text-emerald-400 border-emerald-500/50"} px-3 py-1.5 rounded-lg text-xs font-semibold border transition-all`}
              >
                {isAudioActive ? <><MicOff className="w-3.5 h-3.5" /> Stop Mic</> : <><Mic className="w-3.5 h-3.5" /> Enable Mic</>}
              </button>
          </div>
        </div>

        <div className="p-4 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 items-start max-w-[1600px] mx-auto">
          <div className="flex flex-col gap-6">
            <div className="flex items-center gap-2 mb-1">
              <ListMusic className="w-5 h-5 text-blue-500" />
              <h2 className="text-base font-bold text-neutral-200 uppercase tracking-wider">Playlist</h2>
            </div>
            
            <div className="grid grid-cols-1 gap-1.5">
              {playlist.map((entry, i) => (
                <div 
                  key={i} 
                  className={`flex items-center gap-3 p-1.5 rounded-lg border transition-all ${
                    i === currentIndex 
                      ? "bg-blue-600/10 border-blue-500/50 shadow-[0_0_10px_rgba(59,130,246,0.1)]" 
                      : "bg-neutral-800/50 border-neutral-700/50 hover:border-neutral-600"
                  }`}
                >
                  <div className={`w-6 h-6 rounded flex items-center justify-center font-bold text-[10px] shrink-0 ${i === currentIndex ? "bg-blue-500 text-white" : "bg-neutral-700 text-neutral-400"}`}>
                    {i + 1}
                  </div>
                  <div className="flex-1 min-w-0" onClick={() => entry.path && setCurrentIndex(i)}>
                    <div className={`text-xs font-mono truncate cursor-pointer ${entry.path ? "text-neutral-200" : "text-neutral-500 italic"}`}>
                      {entry.path ? entry.path.split(/[/\\]/).pop() : "Empty Slot"}
                    </div>
                    {entry.midiNote !== undefined && (
                      <div className="text-[9px] text-blue-400 font-bold uppercase tracking-widest leading-tight">
                        Midi Note: {entry.midiNote}
                      </div>
                    )}
                  </div>
                  <button
                    onClick={() => handleSelectSlot(i)}
                    className="p-1.5 hover:bg-neutral-700 rounded text-neutral-400 hover:text-neutral-200 transition-colors"
                  >
                    <FileCode className="w-3.5 h-3.5" />
                  </button>
                </div>
              ))}
            </div>

            <div className="flex gap-2 items-center justify-between bg-neutral-800/80 p-3 rounded-xl border border-neutral-700">
               <button onClick={() => switchIndex(currentIndex - 1)} className="p-1.5 hover:bg-neutral-700 rounded-lg"><ChevronLeft className="w-4 h-4" /></button>
               <span className="text-[10px] font-bold uppercase tracking-widest text-neutral-400">Sketch Switching</span>
               <button onClick={() => switchIndex(currentIndex + 1)} className="p-1.5 hover:bg-neutral-700 rounded-lg"><ChevronRight className="w-4 h-4" /></button>
            </div>
          </div>

          {/* Column 2: Visual Effects */}
          <div className="flex flex-col gap-4">
            <div className="flex items-center gap-2 mb-1">
              <Sparkles className="w-5 h-5 text-indigo-500" />
              <h2 className="text-base font-bold text-neutral-200 uppercase tracking-wider">Effects</h2>
            </div>
            
            <div className="bg-neutral-800/30 p-4 rounded-2xl border border-neutral-800">
              <div className="flex w-full border-b border-neutral-800 mb-4">
                {['bloom', 'rgbShift', 'film', 'vignette'].map((tab) => (
                  <button
                    key={tab}
                    onClick={() => setActiveFxTab(tab as any)}
                    className={`flex-1 pb-2 px-1 text-[10px] font-bold uppercase tracking-wider transition-colors border-b-2 ${activeFxTab === tab ? 'border-blue-500 text-blue-500' : 'border-transparent text-neutral-400'}`}
                  >{tab === 'rgbShift' ? 'RGB' : tab}</button>
                ))}
              </div>
              <div className="min-h-[180px]">
                {activeFxTab === 'bloom' && (
                  <div className="space-y-4 animate-in fade-in duration-200">
                    <div className="space-y-2">
                       <div className="flex justify-between text-xs font-medium"><label className="text-neutral-400">Strength</label><span>{bloomStrength.toFixed(2)}</span></div>
                       <input type="range" min="0" max="3" step="0.01" value={bloomStrength} onChange={(e) => setBloomStrength(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-600" />
                    </div>
                    <div className="space-y-2">
                       <div className="flex justify-between text-xs font-medium"><label className="text-neutral-400">Radius</label><span>{bloomRadius.toFixed(2)}</span></div>
                       <input type="range" min="0" max="1" step="0.01" value={bloomRadius} onChange={(e) => setBloomRadius(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-600" />
                    </div>
                    <div className="space-y-2">
                       <div className="flex justify-between text-xs font-medium"><label className="text-neutral-400">Threshold</label><span>{bloomThreshold.toFixed(2)}</span></div>
                       <input type="range" min="0" max="1" step="0.01" value={bloomThreshold} onChange={(e) => setBloomThreshold(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-600" />
                    </div>
                  </div>
                )}
                {activeFxTab === 'rgbShift' && (
                  <div className="space-y-2">
                    <div className="flex justify-between text-xs font-medium"><label className="text-neutral-400">Amount</label><span>{rgbShiftAmount.toFixed(4)}</span></div>
                    <input type="range" min="0" max="0.05" step="0.0001" value={rgbShiftAmount} onChange={(e) => setRgbShiftAmount(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-fuchsia-500" />
                  </div>
                )}
                {activeFxTab === 'film' && (
                  <div className="space-y-2">
                    <div className="flex justify-between text-xs font-medium"><label className="text-neutral-400">Intensity</label><span>{filmIntensity.toFixed(2)}</span></div>
                    <input type="range" min="0" max="2" step="0.01" value={filmIntensity} onChange={(e) => setFilmIntensity(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-emerald-500" />
                  </div>
                )}
                {activeFxTab === 'vignette' && (
                  <div className="space-y-4">
                    <div className="space-y-2">
                       <div className="flex justify-between text-xs font-medium"><label className="text-neutral-400">Offset</label><span>{vignetteOffset.toFixed(2)}</span></div>
                       <input type="range" min="0" max="3" step="0.01" value={vignetteOffset} onChange={(e) => setVignetteOffset(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-violet-500" />
                    </div>
                    <div className="space-y-2">
                       <div className="flex justify-between text-xs font-medium"><label className="text-neutral-400">Darkness</label><span>{vignetteDarkness.toFixed(2)}</span></div>
                       <input type="range" min="0" max="3" step="0.01" value={vignetteDarkness} onChange={(e) => setVignetteDarkness(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-violet-500" />
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>

          <div className="flex flex-col gap-4">
            {(error || audioError) && (
              <div className="w-full flex items-start gap-2 bg-red-900/20 text-red-400 p-3 rounded-xl text-[10px] border border-red-900/50 animate-in slide-in-from-top-2 mb-2">
                <AlertCircle className="w-4 h-4 shrink-0" />
                <div className="flex flex-col gap-0.5">{error && <p>{error}</p>}{audioError && <p>{audioError}</p>}</div>
              </div>
            )}

            <div className="flex items-center gap-2 mb-1">
              <Activity className="w-5 h-5 text-emerald-500" />
              <h2 className="text-base font-bold text-neutral-200 uppercase tracking-wider">Monitors</h2>
            </div>

            <div className="bg-neutral-800/30 p-4 rounded-2xl border border-neutral-800 flex flex-col gap-4">
              <div className="flex flex-col gap-3">
                <div className="flex items-center gap-2 text-[10px] font-bold text-neutral-400 uppercase tracking-widest leading-none">
                  <Volume2 className="w-3.5 h-3.5 text-orange-400" /> Audio
                </div>
                <div className="grid grid-cols-2 gap-x-4 gap-y-2">
                  <LevelBar label="Vol" value={audioLevels.volume} colorClass="bg-orange-400" />
                  <LevelBar label="Bass" value={audioLevels.bass} colorClass="bg-rose-500" />
                  <LevelBar label="Mid" value={audioLevels.mid} colorClass="bg-amber-500" />
                  <LevelBar label="High" value={audioLevels.high} colorClass="bg-sky-400" />
                </div>
                <div className="mt-1 w-full h-12 bg-neutral-900 rounded-lg overflow-hidden border border-neutral-700/50">
                  <canvas ref={canvasRef} width={256} height={48} className="w-full h-full opacity-80" />
                </div>
              </div>

              <div className="h-px bg-neutral-800 w-full" />

              <div className="flex flex-col gap-2">
                <Indicator label="MIDI" icon={Music} active={isMidiActive || false} text={lastMidi ? lastMidi.text : "Waiting..."} subText={lastMidi ? lastMidi.subText : ""} />
                <Indicator label="OSC" icon={Radio} active={isOscActive || false} text={lastOsc ? lastOsc.text : "Waiting..."} subText={lastOsc ? lastOsc.subText : ""} />
              </div>

              <div className="h-px bg-neutral-800 w-full" />

              <div className="flex flex-col gap-2">
                <div className="flex items-center gap-2 text-[10px] font-bold text-neutral-400 uppercase tracking-widest leading-none">
                  <Eye className="w-3.5 h-3.5 text-blue-400" /> Visualizer Preview
                </div>
                <div className="relative aspect-video w-full bg-neutral-900 rounded-lg overflow-hidden border border-neutral-700/50 flex items-center justify-center group">
                  {previewUrl ? (
                    <img src={previewUrl} alt="Preview" className="w-full h-full object-cover" />
                  ) : (
                    <div className="flex flex-col items-center gap-2 text-neutral-600">
                      <Activity className="w-6 h-6 animate-pulse" />
                      <span className="text-[10px] font-bold uppercase tracking-widest">No Signal</span>
                    </div>
                  )}
                  <div className="absolute top-2 right-2 px-1.5 py-0.5 bg-black/60 backdrop-blur-sm rounded text-[8px] font-bold text-neutral-400 border border-white/10 opacity-0 group-hover:opacity-100 transition-opacity">
                    2 FPS
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
