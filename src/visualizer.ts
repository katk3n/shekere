import * as THREE from 'three';
import { listen, emit } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';
import { ShaderPass } from 'three/examples/jsm/postprocessing/ShaderPass.js';
import { FilmPass } from 'three/examples/jsm/postprocessing/FilmPass.js';
import { RGBShiftShader } from 'three/examples/jsm/shaders/RGBShiftShader.js';
import { VignetteShader } from 'three/examples/jsm/shaders/VignetteShader.js';
import { OutputPass } from 'three/examples/jsm/postprocessing/OutputPass.js';

// Expose THREE globally so user sketches can use it without importing
(window as any).THREE = THREE;

// Shekere API namespace
const Shekere = {
    convertFileSrc,
    clearScene: (container: THREE.Object3D) => clearScene(container),
    SKETCH_DIR: ""
};
(window as any).Shekere = Shekere;



/**
 * Utility to clear all objects from a THREE.Object3D (usually the scene)
 * and dispose of their geometries and materials to prevent memory leaks.
 */
function clearScene(container: THREE.Object3D) {
    while (container.children.length > 0) {
        const object = container.children[0];
        
        object.traverse((child: any) => {
            if (child.isMesh) {
                if (child.geometry) child.geometry.dispose();
                if (child.material) {
                    if (Array.isArray(child.material)) {
                        child.material.forEach((m: any) => m.dispose());
                    } else {
                        child.material.dispose();
                    }
                }
            }
        });

        container.remove(object);
    }
}

interface SketchConfig {
    audio?: {
        minFreqHz?: number;
        maxFreqHz?: number;
    };
    renderer?: {
        toneMapping?: number;
        toneMappingExposure?: number;
    };
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
renderer.setPixelRatio(window.devicePixelRatio); // Better quality on Retina displays

// Revert to NoToneMapping to restore the original "digital" high-contrast look
renderer.toneMapping = THREE.NoToneMapping;
renderer.toneMappingExposure = 1.0;

document.body.appendChild(renderer.domElement);

// --- Post-Processing Setup ---
const renderScene = new RenderPass(scene, camera);

const bloomPass = new UnrealBloomPass(
    new THREE.Vector2(window.innerWidth, window.innerHeight),
    0, // strength
    0, // radius (Default to 0)
    1.0 // threshold (Default to 1.0)
);

const rgbShiftPass = new ShaderPass(RGBShiftShader);
rgbShiftPass.uniforms['amount'].value = 0.0; // Default off

const filmPass = new FilmPass(0.0, false); // Default off

const vignettePass = new ShaderPass(VignetteShader);
vignettePass.uniforms['offset'].value = 0.0; // Default off matches UI default 0
vignettePass.uniforms['darkness'].value = 1.0; // 1.0 means edges target Black

const composer = new EffectComposer(renderer);
composer.addPass(renderScene);
composer.addPass(bloomPass);
composer.addPass(rgbShiftPass);
composer.addPass(filmPass);
composer.addPass(vignettePass);

const outputPass = new OutputPass();
composer.addPass(outputPass);

window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
    composer.setSize(window.innerWidth, window.innerHeight);
});

window.addEventListener('keydown', (e) => {
    emit('visualizer-keydown', { key: e.key }).catch(err => console.error("Keydown emit error:", err));
});

// --- 2. Preview Capture Setup ---
const PREVIEW_WIDTH = 320;
const PREVIEW_INTERVAL_MS = 500; // 2 FPS
const captureCanvas = document.createElement('canvas');
const captureCtx = captureCanvas.getContext('2d');
let lastPreviewTime = 0;

function emitPreviewFrame() {
    if (!captureCtx) return;
    const now = Date.now();
    if (now - lastPreviewTime < PREVIEW_INTERVAL_MS) return;

    // Maintain aspect ratio
    const aspect = window.innerHeight / window.innerWidth;
    const previewHeight = Math.floor(PREVIEW_WIDTH * aspect);
    
    if (captureCanvas.width !== PREVIEW_WIDTH || captureCanvas.height !== previewHeight) {
        captureCanvas.width = PREVIEW_WIDTH;
        captureCanvas.height = previewHeight;
    }

    // Draw the main renderer canvas onto our small capture canvas
    captureCtx.drawImage(renderer.domElement, 0, 0, PREVIEW_WIDTH, previewHeight);
    
    // Convert to low-quality JPEG to minimize IPC payload
    const dataUrl = captureCanvas.toDataURL('image/jpeg', 0.5);
    emit('preview-frame', { dataUrl }).catch(err => console.error("Preview emit error:", err));
    
    lastPreviewTime = now;
}

// --- 3. Audio Analysis (runs locally in this window — no IPC overhead) ---

const FFT_SIZE = 4096;
const BAND_COUNT = 256;
const BASS_MAX_HZ = 250;
const MID_MAX_HZ = 2_000;
const DEFAULT_MIN_FREQ = 27.5;
const DEFAULT_MAX_FREQ = 4186;

let audioContext: AudioContext | null = null;
let analyserNode: AnalyserNode | null = null;
let audioDataArray: Uint8Array | null = null;
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

    analyserNode.getByteFrequencyData(audioDataArray as any);

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

// Listen for Post-Processing settings from Control Panel
listen<{ 
    bloom?: { strength?: number; radius?: number; threshold?: number };
    rgbShift?: { amount?: number };
    film?: { intensity?: number };
    vignette?: { offset?: number; darkness?: number };
}>('update-fx-settings', (event) => {
    const { bloom, rgbShift, film, vignette } = event.payload;
    if (bloom) {
        if (bloom.strength !== undefined) bloomPass.strength = bloom.strength;
        if (bloom.radius !== undefined) bloomPass.radius = bloom.radius;
        if (bloom.threshold !== undefined) bloomPass.threshold = bloom.threshold;
    }
    if (rgbShift) {
        if (rgbShift.amount !== undefined) rgbShiftPass.uniforms['amount'].value = rgbShift.amount;
    }
    if (film) {
        if (film.intensity !== undefined) (filmPass.uniforms as any)['intensity'].value = film.intensity;
    }
    if (vignette) {
        if (vignette.offset !== undefined) vignettePass.uniforms['offset'].value = vignette.offset;
        if (vignette.darkness !== undefined) vignettePass.uniforms['darkness'].value = vignette.darkness;
    }
});

// Legacy listener for backward compatibility during transition
listen<{ strength?: number; radius?: number; threshold?: number }>('update-bloom-settings', (event) => {
    const { strength, radius, threshold } = event.payload;
    if (strength !== undefined) bloomPass.strength = strength;
    if (radius !== undefined) bloomPass.radius = radius;
    if (threshold !== undefined) bloomPass.threshold = threshold;
});

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

        // Proxy objects to allow sketches to modify FX parameters
        const bloomControl = {
            get strength() { return bloomPass.strength; },
            set strength(v) { bloomPass.strength = v; },
            get radius() { return bloomPass.radius; },
            set radius(v) { bloomPass.radius = v; },
            get threshold() { return bloomPass.threshold; },
            set threshold(v) { bloomPass.threshold = v; }
        };

        const rgbShiftControl = {
            get amount() { return rgbShiftPass.uniforms['amount'].value; },
            set amount(v) { rgbShiftPass.uniforms['amount'].value = v; }
        };

        const filmControl = {
            get intensity() { return (filmPass.uniforms as any)['intensity'].value; },
            set intensity(v) { (filmPass.uniforms as any)['intensity'].value = v; }
        };

        const vignetteControl = {
            get offset() { return vignettePass.uniforms['offset'].value; },
            set offset(v) { vignettePass.uniforms['offset'].value = v; },
            get darkness() { return vignettePass.uniforms['darkness'].value; },
            set darkness(v) { vignettePass.uniforms['darkness'].value = v; }
        };

        const context = {
            time,
            audio: latestAudioData,
            midi: latestMidiData,
            osc: latestOscData,
            oscEvents: [...oscEvents],
            bloom: bloomControl,
            rgbShift: rgbShiftControl,
            film: filmControl,
            vignette: vignetteControl
        };
        try {
            currentModule.update(context);
        } catch (e) {
            console.error('Error in update:', e);
        }
        oscEvents.length = 0;
    }

        // Optimizations & Correctness: Disable passes entirely if they have no effect.
        // This prevents visual artifacts and saves GPU power.
        // With OutputPass added at the end, the jump when enabling these is minimized.
        bloomPass.enabled = bloomPass.strength > 0;
        rgbShiftPass.enabled = rgbShiftPass.uniforms['amount'].value > 0;
        filmPass.enabled = (filmPass.uniforms as any)['intensity'].value > 0;
        vignettePass.enabled = vignettePass.uniforms['offset'].value > 0;


    composer.render();

    // Send preview frame at low frequency
    emitPreviewFrame();

    // Throttled sync back to Control Panel
    syncToHost();
}

let lastSyncTime = 0;
let lastSyncedValues = { 
    strength: -1, radius: -1, threshold: -1,
    rgbAmount: -1,
    filmIntensity: -1,
    vignetteOffset: -1, vignetteDarkness: -1
};
const SYNC_THROTTLE_MS = 100; // 10fps sync is enough for UI

function syncToHost() {
    const now = Date.now();
    if (now - lastSyncTime < SYNC_THROTTLE_MS) return;

    const currentValues = {
        strength: bloomPass.strength,
        radius: bloomPass.radius,
        threshold: bloomPass.threshold,
        rgbAmount: rgbShiftPass.uniforms['amount'].value,
        filmIntensity: (filmPass.uniforms as any)['intensity'].value,
        vignetteOffset: vignettePass.uniforms['offset'].value,
        vignetteDarkness: vignettePass.uniforms['darkness'].value
    };

    // Only emit if values have changed significantly
    const hasChanged = 
        Math.abs(currentValues.strength - lastSyncedValues.strength) > 0.001 ||
        Math.abs(currentValues.radius - lastSyncedValues.radius) > 0.001 ||
        Math.abs(currentValues.threshold - lastSyncedValues.threshold) > 0.001 ||
        Math.abs(currentValues.rgbAmount - lastSyncedValues.rgbAmount) > 0.0001 ||
        Math.abs(currentValues.filmIntensity - lastSyncedValues.filmIntensity) > 0.001 ||
        Math.abs(currentValues.vignetteOffset - lastSyncedValues.vignetteOffset) > 0.001 ||
        Math.abs(currentValues.vignetteDarkness - lastSyncedValues.vignetteDarkness) > 0.001;

    if (hasChanged) {
        emit('fx-settings-changed', {
            bloom: { strength: currentValues.strength, radius: currentValues.radius, threshold: currentValues.threshold },
            rgbShift: { amount: currentValues.rgbAmount },
            film: { intensity: currentValues.filmIntensity },
            vignette: { offset: currentValues.vignetteOffset, darkness: currentValues.vignetteDarkness }
        });
        lastSyncedValues = { ...currentValues };
    }

    // Always emit audio activity if active (throttled to 10fps by this function)
    if (audioActive && latestAudioData) {
        emit('audio-activity', {
            volume: latestAudioData.volume || 0,
            bass: latestAudioData.bass || 0,
            mid: latestAudioData.mid || 0,
            high: latestAudioData.high || 0,
            bands: latestAudioData.bands || []
        }).catch(err => console.error("Audio activity emit error:", err));
    }

    lastSyncTime = now;
}

animate();

// --- 5. Dynamic Module Loader ---
listen<{ code: string; dir?: string }>('user-code-update', async (event) => {
    try {
        const { code: jsCode, dir } = event.payload;

        // Update sketch directory for relative path resolution
        if (dir) Shekere.SKETCH_DIR = dir;

        const blob = new Blob([jsCode], { type: 'application/javascript' });
        const blobUrl = URL.createObjectURL(blob);

        if (currentModule && typeof currentModule.cleanup === 'function') {
            try { currentModule.cleanup(scene); } catch (e) { console.warn('Cleanup failed:', e); }
        }

        const userModule = await import(/* @vite-ignore */ blobUrl);
        const sketchContext = {};

        if (typeof userModule.setup === 'function') {
            const config = userModule.setup.call(sketchContext, scene);
            
            // Default renderer state if not specified by sketch
            renderer.toneMapping = THREE.NoToneMapping;
            renderer.toneMappingExposure = 1.0;
            scene.background = null;

            if (config) {
                if (config.audio) {
                    applyAudioConfig(config.audio);
                }
                if (config.renderer) {
                    if (config.renderer.toneMapping !== undefined) renderer.toneMapping = config.renderer.toneMapping;
                    if (config.renderer.toneMappingExposure !== undefined) renderer.toneMappingExposure = config.renderer.toneMappingExposure;
                }
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
