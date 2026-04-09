import * as THREE from 'three';
import { listen } from '@tauri-apps/api/event';

// Expose THREE globally so user sketches can use it without importing
(window as any).THREE = THREE;

interface SketchConfig {
    audio?: {
        minFreqHz?: number;
        maxFreqHz?: number;
    }
}

// Type definition for user-provided sketch modules
interface SketchModule {
    setup?: (scene: THREE.Scene) => SketchConfig | void;
    update?: (context: any) => void;
    cleanup?: (scene: THREE.Scene) => void;
}

// --- 1. Three.js Basic Setup ---
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
camera.position.z = 5;

const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setSize(window.innerWidth, window.innerHeight);
renderer.setClearColor(0x000000, 1);
document.body.appendChild(renderer.domElement);

window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
});

// --- 2. Audio Analysis (runs locally in this window — no IPC overhead) ---

const FFT_SIZE = 4096;
const BAND_COUNT = 256;
const BASS_MAX_HZ = 250;
const MID_MAX_HZ = 2_000;
const DEFAULT_MIN_FREQ = 27.5;
const DEFAULT_MAX_FREQ = 4186;

let audioContext: AudioContext | null = null;
let analyserNode: AnalyserNode | null = null;
let audioDataArray: Uint8Array<ArrayBuffer> | null = null;
let audioStream: MediaStream | null = null;
let audioActive = false;
let audioMinFreq = DEFAULT_MIN_FREQ;
let audioMaxFreq = DEFAULT_MAX_FREQ;

function applyAudioConfig(config: { minFreqHz?: number; maxFreqHz?: number }) {
    if (config.minFreqHz !== undefined) audioMinFreq = config.minFreqHz;
    if (config.maxFreqHz !== undefined) audioMaxFreq = config.maxFreqHz;
    console.log(`Audio config updated: ${audioMinFreq}Hz - ${audioMaxFreq}Hz`);
}

async function startAudio() {
    if (audioActive) return;
    try {
        audioStream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
        const AudioContextCtor = window.AudioContext || (window as any).webkitAudioContext;
        audioContext = new AudioContextCtor();
        analyserNode = audioContext.createAnalyser();
        analyserNode.fftSize = FFT_SIZE;
        analyserNode.smoothingTimeConstant = 0.5;
        analyserNode.minDecibels = -70;
        analyserNode.maxDecibels = -10;
        audioDataArray = new Uint8Array(new ArrayBuffer(analyserNode.frequencyBinCount));
        const source = audioContext.createMediaStreamSource(audioStream);
        source.connect(analyserNode);
        audioActive = true;
        console.log('Audio capture started in Visualizer.');
    } catch (e) {
        console.error('Failed to start audio capture:', e);
    }
}

function stopAudio() {
    audioActive = false;
    if (audioStream) {
        audioStream.getTracks().forEach(t => t.stop());
        audioStream = null;
    }
    if (audioContext) {
        audioContext.close().catch(console.error);
        audioContext = null;
    }
    analyserNode = null;
    audioDataArray = null;
    console.log('Audio capture stopped.');
}

function computeAudioData() {
    if (!analyserNode || !audioDataArray) {
        return { volume: 0, bass: 0, mid: 0, high: 0, bands: new Array(BAND_COUNT).fill(0) as number[] };
    }

    analyserNode.getByteFrequencyData(audioDataArray);

    const sampleRate = audioContext?.sampleRate ?? 44100;
    const binResolution = sampleRate / FFT_SIZE;
    const minFreq = audioMinFreq;
    const maxFreq = audioMaxFreq;
    const logRatio = Math.pow(maxFreq / minFreq, 1 / BAND_COUNT);

    const bands: number[] = new Array(BAND_COUNT);
    for (let b = 0; b < BAND_COUNT; b++) {
        const freqStart = minFreq * Math.pow(logRatio, b);
        const freqEnd = minFreq * Math.pow(logRatio, b + 1);
        const binStart = Math.floor(freqStart / binResolution);
        const binEnd = Math.max(binStart + 1, Math.floor(freqEnd / binResolution));

        let sum = 0, count = 0;
        for (let i = binStart; i < binEnd && i < audioDataArray.length; i++) {
            sum += audioDataArray[i];
            count++;
        }

        let val = count > 0 ? (sum / count) / 255.0 : 0;
        // Tilt EQ: boost high frequencies (1.0x → 1.8x)
        val *= 1.0 + (b / BAND_COUNT) * 0.8;
        // Non-linear scaling: suppress noise, emphasize clear sounds
        bands[b] = Math.min(1.0, Math.pow(val, 1.5));
    }

    const getIdx = (f: number) => {
        if (f <= minFreq) return 0;
        if (f >= maxFreq) return BAND_COUNT;
        return Math.floor(Math.log(f / minFreq) / Math.log(logRatio));
    };

    const bassEnd = getIdx(BASS_MAX_HZ);
    const midEnd = getIdx(MID_MAX_HZ);

    const avgRange = (arr: number[], from: number, to: number) => {
        const s = Math.max(0, from);
        const e = Math.min(arr.length, to);
        if (s >= e) return 0;
        let sum = 0;
        for (let i = s; i < e; i++) sum += arr[i];
        return sum / (e - s);
    };

    return {
        volume: bands.reduce((a, b) => a + b, 0) / BAND_COUNT,
        bass: avgRange(bands, 0, bassEnd),
        mid: avgRange(bands, bassEnd, midEnd),
        high: avgRange(bands, midEnd, BAND_COUNT),
        bands,
    };
}

// Listen for start/stop commands from the Control Panel
listen<void>('start-audio', () => { startAudio(); });
listen<void>('stop-audio', () => { stopAudio(); });

// --- 3. Shared state ---
let currentModule: SketchModule | null = null;
let latestAudioData = { volume: 0, bass: 0, mid: 0, high: 0, bands: new Array(BAND_COUNT).fill(0) as number[] };
let latestMidiData = {
    notes: new Array(128).fill(0) as number[],
    cc: new Array(128).fill(0) as number[]
};
let latestOscData: Record<string, any> = {};
let oscEvents: { address: string; data: any }[] = [];

listen<{ status: number; data1: number; data2: number }>('midi-event', (event) => {
    const { status, data1, data2 } = event.payload;
    const type = status & 0xF0;
    if (type === 0x90) {
        latestMidiData.notes[data1] = data2 / 127.0;
    } else if (type === 0x80) {
        latestMidiData.notes[data1] = 0;
    } else if (type === 0xB0) {
        latestMidiData.cc[data1] = data2 / 127.0;
    }
});

listen<{ address: string; args: any[] }>('osc-event', (event) => {
    const { address, args } = event.payload;
    let data: any = args;
    if (address === '/dirt/play' && args.length % 2 === 0) {
        const obj: Record<string, any> = {};
        for (let i = 0; i < args.length; i += 2) {
            obj[String(args[i])] = args[i + 1];
        }
        data = obj;
    }
    latestOscData[address] = data;
    oscEvents.push({ address, data });
});

// --- 4. Render Loop ---
const clock = new THREE.Clock();
function animate() {
    requestAnimationFrame(animate);

    // Compute audio data locally on every frame (no IPC)
    if (audioActive) {
        latestAudioData = computeAudioData();
    }

    if (currentModule && typeof currentModule.update === 'function') {
        const time = clock.getElapsedTime();
        const context = {
            time,
            audio: latestAudioData,
            midi: latestMidiData,
            osc: latestOscData,
            oscEvents: [...oscEvents]
        };
        try {
            currentModule.update(context);
        } catch (e) {
            console.error('Error in update:', e);
        }
        oscEvents.length = 0;
    }

    renderer.render(scene, camera);
}
animate();

// --- 5. Dynamic Module Loader ---
listen<{ code: string }>('user-code-update', async (event) => {
    try {
        const jsCode = event.payload.code;
        const blob = new Blob([jsCode], { type: 'application/javascript' });
        const blobUrl = URL.createObjectURL(blob);

        if (currentModule && typeof currentModule.cleanup === 'function') {
            try { currentModule.cleanup(scene); } catch (e) { console.warn('Cleanup failed:', e); }
        }

        const userModule = await import(/* @vite-ignore */ blobUrl);
        const sketchContext = {};

        if (typeof userModule.setup === 'function') {
            const config = userModule.setup.call(sketchContext, scene);
            // Apply audio config directly — no cross-window IPC needed
            if (config && config.audio) {
                applyAudioConfig(config.audio);
            }
        }

        currentModule = {
            update: (ctx: any) => userModule.update?.call(sketchContext, ctx),
            cleanup: (s: any) => userModule.cleanup?.call(sketchContext, s)
        };

        console.log('Successfully hot-reloaded user code.');
        URL.revokeObjectURL(blobUrl);
    } catch (e: any) {
        console.error('Failed to execute user sketch:', e);
    }
});
