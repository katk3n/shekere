import { useState, useEffect, useRef } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { readTextFile, watch } from "@tauri-apps/plugin-fs";
import { emit, listen } from "@tauri-apps/api/event";
import { FileCode, AlertCircle, FileAudio, Settings, Mic, MicOff, Sparkles, Activity, Radio, Music, Volume2 } from "lucide-react";
import { useAudioAnalyzer } from "./hooks/useAudioAnalyzer";

// --- Helper Components ---
const LevelBar = ({ label, value, colorClass }: { label: string, value: number, colorClass: string }) => (
  <div className="flex flex-col gap-1 w-full">
    <div className="flex justify-between items-center text-[10px] sm:text-xs font-semibold text-neutral-500 uppercase tracking-wider">
      <span>{label}</span>
      <span className="text-neutral-400 font-mono">{(value * 100).toFixed(0)}%</span>
    </div>
    <div className="h-2 w-full bg-neutral-200 dark:bg-neutral-700/50 rounded-full overflow-hidden">
      <div 
        className={`h-full ${colorClass} transition-all duration-[50ms] ease-out min-w-[2%]`}
        style={{ width: `${Math.min(Math.max(value * 100, 0), 100)}%` }}
      />
    </div>
  </div>
);

const Indicator = ({ label, icon: Icon, active, text }: { label: string, icon: any, active: boolean, text: string }) => (
  <div className="flex items-center gap-3 bg-white dark:bg-neutral-800 p-3 rounded-lg border border-neutral-100 dark:border-neutral-700/50 shadow-sm">
    <div className="relative flex items-center justify-center">
      <Icon className="w-5 h-5 text-neutral-400 dark:text-neutral-500" />
      <div className={`absolute -top-1 -right-1 w-2.5 h-2.5 rounded-full transition-colors duration-[50ms] ${active ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.8)]' : 'bg-neutral-300 dark:bg-neutral-700'}`} />
    </div>
    <div className="flex flex-col flex-1 min-w-0">
      <span className="text-[10px] text-neutral-500 dark:text-neutral-400 uppercase tracking-widest font-semibold">{label}</span>
      <span className="text-sm font-mono text-neutral-800 dark:text-neutral-200 truncate" title={text}>{text}</span>
    </div>
  </div>
);

export default function App() {
  const [filePath, setFilePath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { isActive: isAudioActive, start: startAudio, stop: stopAudio, error: audioError } = useAudioAnalyzer();
  
  // Bloom Settings State (Default to 0 / No effect)
  const [bloomStrength, setBloomStrength] = useState(0);
  const [bloomRadius, setBloomRadius] = useState(0);
  const [bloomThreshold, setBloomThreshold] = useState(1.0);

  // New FX Settings State
  const [rgbShiftAmount, setRgbShiftAmount] = useState(0);
  const [filmIntensity, setFilmIntensity] = useState(0);
  const [vignetteOffset, setVignetteOffset] = useState(0); // 0 completely disables vignette
  const [vignetteDarkness, setVignetteDarkness] = useState(1.0); // 1.0 ensures edges target black, not white

  const [activeFxTab, setActiveFxTab] = useState<'bloom' | 'rgbShift' | 'film' | 'vignette'>('bloom');

  // Signal Activity States
  const [audioLevels, setAudioLevels] = useState({ volume: 0, bass: 0, mid: 0, high: 0 });
  const [lastMidi, setLastMidi] = useState<{ text: string, id: number } | null>(null);
  const [lastOsc, setLastOsc] = useState<{ text: string, id: number } | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Listeners for Signal Activity
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
          ctx.fillStyle = '#10b981'; // emerald-500
          const bands = e.payload.bands;
          const barWidth = w / bands.length;
          for (let i = 0; i < bands.length; i++) {
            const barHeight = bands[i] * h;
            // Draw slightly separated bars if possible, or continuous
            ctx.fillRect(i * barWidth, h - barHeight, Math.max(1, barWidth - 0.5), barHeight);
          }
        }
      }
    });
    const u2 = listen<any>('midi-event', (e) => {
      const p = e.payload;
      const typeStr = p.message_type === 144 ? 'NoteOn' : p.message_type === 128 ? 'NoteOff' : p.message_type === 176 ? 'CC' : 'MIDI';
      setLastMidi({ text: `${typeStr} [${p.data1}] val: ${p.data2}`, id: Date.now() });
    });
    const u3 = listen<any>('osc-event', (e) => {
      setLastOsc({ text: e.payload.address, id: Date.now() });
    });
    return () => {
      u1.then(f => f()); u2.then(f => f()); u3.then(f => f());
    };
  }, []);


  // Ref to prevent feedback loops when syncing from Visualizer
  const skipNextEmitRef = { current: false };

  const handleToggleAudio = () => {
    if (isAudioActive) stopAudio();
    else startAudio();
  };

  // Sync FX settings to Visualizer
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

  // Listen for sync from Visualizer (driven by sketch code/MIDI)
  useEffect(() => {
    const unlistenPromise = listen<any>(
      "fx-settings-changed",
      (event) => {
        const { bloom, rgbShift, film, vignette } = event.payload;
        
        // We only set skipNextEmitRef true once before setting all states
        // But React batches these sets, so one flag is sufficient
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

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!filePath) return;

    let unwatch: (() => void) | null = null;
    let lastEmitTime = 0;
    const THROTTLE_MS = 150;

    const loadAndEmit = async () => {
      try {
        const code = await readTextFile(filePath);
        await emit("user-code-update", { code });
        setError(null);
      } catch (err: any) {
        console.error("Failed to read or emit file:", err);
        setError(`Failed to read file: ${err.message || err}`);
      }
    };

    // Initial load
    loadAndEmit();

    // Start watching
    watch(
      filePath,
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

    return () => {
      if (unwatch) unwatch();
    };
  }, [filePath]);

  const handleOpenFile = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: "JavaScript", extensions: ["js"] }],
      });

      if (selected && typeof selected === "string") {
        setFilePath(selected);
      }
    } catch (err: any) {
      console.error("Dialog error:", err);
      setError(`Dialog failed: ${err.message || err}`);
    }
  };

  // Logic to determine if MIDI/OSC indicators should flash active
  const isMidiActive = lastMidi && (Date.now() - lastMidi.id < 150);
  const isOscActive = lastOsc && (Date.now() - lastOsc.id < 150);

  return (
    <div className="min-h-screen bg-neutral-100 dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 flex flex-col items-center justify-center p-6 font-sans transition-colors duration-200">
      <div className="max-w-[850px] w-full bg-white dark:bg-neutral-800 rounded-2xl shadow-xl overflow-hidden border border-neutral-200 dark:border-neutral-700">
        
        <div className="p-6 border-b border-neutral-200 dark:border-neutral-700 flex items-center gap-3">
          <Settings className="w-6 h-6 text-blue-500" />
          <h1 className="text-2xl font-bold tracking-tight">shekere Control Panel</h1>
        </div>

        <div className="p-8 grid grid-cols-1 md:grid-cols-2 gap-8 items-start">
          
          {/* ================= LEFT COLUMN: CONTROLS ================= */}
          <div className="flex flex-col gap-6">
            <div className="w-full flex gap-3">
              <button
                onClick={handleOpenFile}
                className="flex-1 flex justify-center items-center gap-2 bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white px-4 py-3.5 rounded-xl font-medium transition-all shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <FileCode className="w-5 h-5" />
                Select JS
              </button>
              <button
                onClick={handleToggleAudio}
                className={`flex-1 flex justify-center items-center gap-2 ${
                  isAudioActive
                    ? "bg-red-500 hover:bg-red-600 active:bg-red-700 text-white"
                    : "bg-emerald-500 hover:bg-emerald-600 active:bg-emerald-700 text-white"
                } px-4 py-3.5 rounded-xl font-medium transition-all shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-neutral-800`}
              >
                {isAudioActive ? <><MicOff className="w-5 h-5" />Stop Mic</> : <><Mic className="w-5 h-5" />Enable Mic</>}
              </button>
            </div>

            {/* Post-Processing Section */}
            <div className="w-full flex flex-col pt-2">
              <div className="flex items-center gap-2 mb-4">
                <Sparkles className="w-5 h-5 text-indigo-500" />
                <h2 className="text-base font-bold text-neutral-800 dark:text-neutral-200 tracking-tight">
                  Visual Effects
                </h2>
              </div>
              
              {/* Tabs Header */}
              <div className="flex w-full overflow-x-auto hide-scrollbar border-b border-neutral-200 dark:border-neutral-800 mb-4 pt-1">
                <button 
                  onClick={() => setActiveFxTab('bloom')}
                  className={`flex-1 pb-2 px-1 text-[10px] sm:text-xs font-bold uppercase tracking-wider whitespace-nowrap transition-colors border-b-2 ${activeFxTab === 'bloom' ? 'border-blue-500 text-blue-500' : 'border-transparent text-neutral-400 hover:text-neutral-300'}`}
                >Bloom</button>
                <button 
                  onClick={() => setActiveFxTab('rgbShift')}
                  className={`flex-1 pb-2 px-1 text-[10px] sm:text-xs font-bold uppercase tracking-wider whitespace-nowrap transition-colors border-b-2 ${activeFxTab === 'rgbShift' ? 'border-fuchsia-500 text-fuchsia-500' : 'border-transparent text-neutral-400 hover:text-neutral-300'}`}
                >RGB</button>
                <button 
                  onClick={() => setActiveFxTab('film')}
                  className={`flex-1 pb-2 px-1 text-[10px] sm:text-xs font-bold uppercase tracking-wider whitespace-nowrap transition-colors border-b-2 ${activeFxTab === 'film' ? 'border-emerald-500 text-emerald-500' : 'border-transparent text-neutral-400 hover:text-neutral-300'}`}
                >Film</button>
                <button 
                  onClick={() => setActiveFxTab('vignette')}
                  className={`flex-1 pb-2 px-1 text-[10px] sm:text-xs font-bold uppercase tracking-wider whitespace-nowrap transition-colors border-b-2 ${activeFxTab === 'vignette' ? 'border-violet-500 text-violet-500' : 'border-transparent text-neutral-400 hover:text-neutral-300'}`}
                >Vignette</button>
              </div>

              {/* Tab Content Container */}
              <div className="bg-neutral-50 dark:bg-neutral-800/50 p-4 rounded-xl border border-neutral-100 dark:border-neutral-700/50 min-h-[160px]">
                
                {/* Bloom */}
                {activeFxTab === 'bloom' && (
                  <div className="space-y-4 animate-in fade-in duration-200">
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-xs font-medium">
                        <label className="text-neutral-600 dark:text-neutral-400">Strength</label>
                        <span className="text-neutral-900 dark:text-neutral-100">{bloomStrength.toFixed(2)}</span>
                      </div>
                      <input type="range" min="0" max="3" step="0.01" value={bloomStrength} onChange={(e) => setBloomStrength(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-200 dark:bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-600" />
                    </div>
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-xs font-medium">
                        <label className="text-neutral-600 dark:text-neutral-400">Radius</label>
                        <span className="text-neutral-900 dark:text-neutral-100">{bloomRadius.toFixed(2)}</span>
                      </div>
                      <input type="range" min="0" max="1" step="0.01" value={bloomRadius} onChange={(e) => setBloomRadius(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-200 dark:bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-600" />
                    </div>
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-xs font-medium">
                        <label className="text-neutral-600 dark:text-neutral-400">Threshold</label>
                        <span className="text-neutral-900 dark:text-neutral-100">{bloomThreshold.toFixed(2)}</span>
                      </div>
                      <input type="range" min="0" max="1" step="0.01" value={bloomThreshold} onChange={(e) => setBloomThreshold(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-200 dark:bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-blue-600" />
                    </div>
                  </div>
                )}

                {/* RGB Shift */}
                {activeFxTab === 'rgbShift' && (
                  <div className="space-y-4 animate-in fade-in duration-200">
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-xs font-medium">
                        <label className="text-neutral-600 dark:text-neutral-400">Amount</label>
                        <span className="text-neutral-900 dark:text-neutral-100">{rgbShiftAmount.toFixed(4)}</span>
                      </div>
                      <input type="range" min="0" max="0.05" step="0.0001" value={rgbShiftAmount} onChange={(e) => setRgbShiftAmount(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-200 dark:bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-fuchsia-500" />
                    </div>
                  </div>
                )}

                {/* Film Grain */}
                {activeFxTab === 'film' && (
                  <div className="space-y-4 animate-in fade-in duration-200">
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-xs font-medium">
                        <label className="text-neutral-600 dark:text-neutral-400">Intensity</label>
                        <span className="text-neutral-900 dark:text-neutral-100">{filmIntensity.toFixed(2)}</span>
                      </div>
                      <input type="range" min="0" max="2" step="0.01" value={filmIntensity} onChange={(e) => setFilmIntensity(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-200 dark:bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-emerald-500" />
                    </div>
                  </div>
                )}

                {/* Vignette */}
                {activeFxTab === 'vignette' && (
                  <div className="space-y-4 animate-in fade-in duration-200">
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-xs font-medium">
                        <label className="text-neutral-600 dark:text-neutral-400">Offset</label>
                        <span className="text-neutral-900 dark:text-neutral-100">{vignetteOffset.toFixed(2)}</span>
                      </div>
                      <input type="range" min="0" max="3" step="0.01" value={vignetteOffset} onChange={(e) => setVignetteOffset(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-200 dark:bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-violet-500" />
                    </div>
                    <div className="space-y-1.5">
                      <div className="flex justify-between text-xs font-medium">
                        <label className="text-neutral-600 dark:text-neutral-400">Darkness</label>
                        <span className="text-neutral-900 dark:text-neutral-100">{vignetteDarkness.toFixed(2)}</span>
                      </div>
                      <input type="range" min="0" max="3" step="0.01" value={vignetteDarkness} onChange={(e) => setVignetteDarkness(parseFloat(e.target.value))} className="w-full h-1.5 bg-neutral-200 dark:bg-neutral-700 rounded-lg appearance-none cursor-pointer accent-violet-500" />
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>


          {/* ================= RIGHT COLUMN: MONITORS ================= */}
          <div className="flex flex-col gap-6">
            
            {/* Currently Watching */}
            <div className="w-full flex flex-col gap-2">
              <h2 className="text-sm font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">
                Currently Watching
              </h2>
              <div className="bg-neutral-100 dark:bg-neutral-900/50 p-4 rounded-xl text-sm font-mono break-all text-neutral-700 dark:text-neutral-300 border border-neutral-200 dark:border-neutral-700/50 flex items-start gap-3">
                <FileAudio className="w-5 h-5 shrink-0 text-neutral-400 mt-0.5" />
                <span>{filePath || "None"}</span>
              </div>
            </div>

            {/* Error Displays */}
            {(error || audioError) && (
              <div className="w-full flex items-start gap-3 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 p-4 rounded-xl text-sm border border-red-200 dark:border-red-900/50">
                <AlertCircle className="w-5 h-5 shrink-0" />
                <div className="flex flex-col gap-1">
                  {error && <p>{error}</p>}
                  {audioError && <p>{audioError}</p>}
                </div>
              </div>
            )}

            {/* Signal Monitors */}
            <div className="w-full flex flex-col border-t border-neutral-200 dark:border-neutral-700 pt-6 mt-2">
              <div className="flex items-center gap-2 mb-4">
                <Activity className="w-5 h-5 text-emerald-500" />
                <h2 className="text-base font-bold text-neutral-800 dark:text-neutral-200 tracking-tight">
                  Signal Monitors
                </h2>
              </div>
              
              <div className="flex flex-col gap-4 bg-neutral-50 dark:bg-neutral-900/30 p-5 rounded-xl border border-neutral-100 dark:border-neutral-800">
                
                {/* Audio Levels */}
                <div className="flex flex-col gap-3">
                  <div className="flex items-center gap-2 text-sm font-semibold text-neutral-700 dark:text-neutral-300 mb-1">
                    <Volume2 className="w-4 h-4 text-orange-400" />
                    Audio Levels
                  </div>
                  <LevelBar label="Volume" value={audioLevels.volume} colorClass="bg-orange-400" />
                  <LevelBar label="Bass" value={audioLevels.bass} colorClass="bg-rose-500" />
                  <LevelBar label="Mid" value={audioLevels.mid} colorClass="bg-amber-500" />
                  <LevelBar label="High" value={audioLevels.high} colorClass="bg-sky-400" />
                  
                  {/* Spectrum Canvas */}
                  <div className="mt-1 w-full h-16 bg-neutral-200 dark:bg-neutral-800 rounded-lg overflow-hidden border border-neutral-300 dark:border-neutral-700/50">
                    <canvas ref={canvasRef} width={256} height={64} className="w-full h-full opacity-80" />
                  </div>
                </div>

                <div className="w-full h-px bg-neutral-200 dark:bg-neutral-800 my-2" />

                {/* MIDI & OSC Indicators */}
                <div className="flex flex-col gap-3">
                  <Indicator 
                    label="MIDI IN" 
                    icon={Music} 
                    active={isMidiActive || false} 
                    text={lastMidi ? lastMidi.text : "Waiting for signal..."} 
                  />
                  <Indicator 
                    label="OSC IN (2020)" 
                    icon={Radio} 
                    active={isOscActive || false} 
                    text={lastOsc ? lastOsc.text : "Waiting for signal..."} 
                  />
                </div>
                
              </div>
            </div>

          </div>

        </div>
      </div>
    </div>
  );
}
