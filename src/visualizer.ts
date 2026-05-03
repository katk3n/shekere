import * as THREE from 'three';
import { listen, emit } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';

import Meyda from 'meyda';
import { WebGPURenderer, RenderPipeline, MeshStandardNodeMaterial, PointsNodeMaterial, LineBasicNodeMaterial, MeshBasicNodeMaterial, NodeMaterial } from 'three/webgpu';
import * as TSL from 'three/tsl';
import { bloom } from 'three/addons/tsl/display/BloomNode.js';
import { film } from 'three/addons/tsl/display/FilmNode.js';
import { rgbShift } from 'three/addons/tsl/display/RGBShiftNode.js';

// Expose THREE globally so user sketches can use it without importing
// Clone the namespace to avoid "assign to readonly property" errors
(window as any).THREE = {
    ...THREE,
    MeshStandardNodeMaterial,
    PointsNodeMaterial,
    LineBasicNodeMaterial,
    MeshBasicNodeMaterial,
    NodeMaterial
};

// Expose TSL
(window as any).TSL = {
    ...TSL,
    bloom,
    film,
    rgbShift
};

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
        features?: string[];
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

// WebGPURenderer is required for TSL
const renderer = new WebGPURenderer({ antialias: true });
renderer.setSize(window.innerWidth, window.innerHeight);
renderer.setClearColor(0x000000, 1);
renderer.setPixelRatio(window.devicePixelRatio); // Better quality on Retina displays

// Revert to NoToneMapping to restore the original "digital" high-contrast look
renderer.toneMapping = THREE.NoToneMapping;
renderer.toneMappingExposure = 1.0;

document.body.appendChild(renderer.domElement);

// --- Post-Processing Setup ---
const postProcessing = new RenderPipeline(renderer);

// FX variables (initial primitive values)
let currentVignetteOffset = 0.0;
let currentVignetteDarkness = 1.0;

// Base scene pass
let scenePass = TSL.pass(scene, camera);

// Standard Three.js Eskil's Vignette
const applyVignette = (colorNode: any, offset: any, darkness: any) => {
    const uv = TSL.uv();
    // vec2 uvOffset = ( uv - vec2( 0.5 ) ) * vec2( offset );
    const uvOffset = uv.sub(TSL.vec2(0.5)).mul(offset);
    
    // dot( uvOffset, uvOffset )
    const distSq = TSL.dot(uvOffset, uvOffset);
    
    // vec3( 1.0 - darkness )
    const darkColor = TSL.vec3(TSL.sub(1.0, darkness));
    
    // mix( texel.rgb, vec3( 1.0 - darkness ), dot( uv, uv ) )
    // Ensure we clamp the mix factor so it doesn't invert colors outside the circle
    const mixFactor = TSL.clamp(distSq, 0.0, 1.0);
    const mixed = TSL.mix(colorNode.rgb, darkColor, mixFactor);
    
    return TSL.vec4(mixed, colorNode.a);
};

// Pipeline composition
// 1. Scene
let currentPass = scenePass.getTextureNode('output') as any;
// 2. Bloom
const bloomNode = bloom(currentPass, 0.0 as any, 0.0 as any, 1.0 as any);
// BloomNode returns ONLY the bloom effect (blurred highlights), so we must ADD it to the scene.
currentPass = currentPass.add(bloomNode as any);

// 3. RGB Shift
const rgbNode = rgbShift(currentPass, 0.0 as any);
// We use a custom uniform for mixing so we can dynamically toggle it
const rgbMix = TSL.uniform(0.0);
currentPass = TSL.mix(currentPass, rgbNode as any, rgbMix.greaterThan(0.0) as any);

// 4. Film
const filmIntensityUniform = TSL.uniform(0.0);
const filmNode = film(currentPass, filmIntensityUniform);
const filmMix = TSL.uniform(0.0);
currentPass = TSL.mix(currentPass, filmNode as any, filmMix.greaterThan(0.0) as any);

// 5. Vignette (Custom TSL)
const vignetteOffsetUniform = TSL.uniform(1.0);
const vignetteDarknessUniform = TSL.uniform(1.0);
const vignetteNode = applyVignette(currentPass, vignetteOffsetUniform, vignetteDarknessUniform);
// Mix it using a threshold on darkness/offset if needed, but the formula inherently works.
currentPass = vignetteNode;

// Final Output
postProcessing.outputNode = currentPass;

window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
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
const CORE_FEATURES = ['rms', 'zcr', 'energy', 'spectralCentroid', 'spectralFlatness', 'chroma', 'mfcc'];

let audioContext: AudioContext | null = null;
let analyserNode: AnalyserNode | null = null;
let audioSourceNode: MediaStreamAudioSourceNode | null = null;
let audioDataArray: Uint8Array | null = null;
let audioStream: MediaStream | null = null;
let audioActive = false;
let audioMinFreq = DEFAULT_MIN_FREQ;
let audioMaxFreq = DEFAULT_MAX_FREQ;

let meydaAnalyzer: any = null;

function applyAudioConfig(config: { minFreqHz?: number; maxFreqHz?: number; features?: string[] }) {
    if (config.minFreqHz !== undefined) audioMinFreq = config.minFreqHz;
    if (config.maxFreqHz !== undefined) audioMaxFreq = config.maxFreqHz;
    // features opt-in is no longer needed as we always extract CORE_FEATURES
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
        audioSourceNode = audioContext.createMediaStreamSource(audioStream);
        audioSourceNode.connect(analyserNode);
        
        // Always initialize Meyda with all core features
        meydaAnalyzer = Meyda.createMeydaAnalyzer({
            audioContext: audioContext,
            source: audioSourceNode,
            bufferSize: FFT_SIZE,
            featureExtractors: CORE_FEATURES
        });
        
        audioActive = true;
        console.log('Audio capture started in Visualizer (Meyda enabled).');
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
    audioSourceNode = null;
    audioDataArray = null;
    meydaAnalyzer = null;
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

    let features = {};
    if (meydaAnalyzer) {
        try {
            features = meydaAnalyzer.get(CORE_FEATURES) || {};
        } catch (e) {
            console.warn('Meyda feature extraction error:', e);
        }
    }

    return {
        volume: bands.reduce((a, b) => a + b, 0) / BAND_COUNT,
        bass: avgRange(bands, 0, bassEnd),
        mid: avgRange(bands, bassEnd, midEnd),
        high: avgRange(bands, midEnd, BAND_COUNT),
        bands,
        features
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
        if (bloom.strength !== undefined) (bloomNode as any).strength.value = bloom.strength;
        if (bloom.radius !== undefined) (bloomNode as any).radius.value = bloom.radius;
        if (bloom.threshold !== undefined) (bloomNode as any).threshold.value = bloom.threshold;
    }
    if (rgbShift) {
        if (rgbShift.amount !== undefined) {
            (rgbNode as any).amount.value = rgbShift.amount;
            rgbMix.value = rgbShift.amount;
        }
    }
    if (film) {
        if (film.intensity !== undefined) {
            filmIntensityUniform.value = film.intensity;
            filmMix.value = film.intensity;
        }
    }
    if (vignette) {
        if (vignette.offset !== undefined) {
            currentVignetteOffset = vignette.offset;
            vignetteOffsetUniform.value = vignette.offset;
        }
        if (vignette.darkness !== undefined) {
            currentVignetteDarkness = vignette.darkness;
            vignetteDarknessUniform.value = vignette.darkness;
        }
    }
});

// Legacy listener for backward compatibility during transition
listen<{ strength?: number; radius?: number; threshold?: number }>('update-bloom-settings', (event) => {
    const { strength, radius, threshold } = event.payload;
    if (strength !== undefined) (bloomNode as any).strength.value = strength;
    if (radius !== undefined) (bloomNode as any).radius.value = radius;
    if (threshold !== undefined) (bloomNode as any).threshold.value = threshold;
});

// --- 3. Shared state ---
let currentModule: SketchModule | null = null;
let latestAudioData: any = { volume: 0, bass: 0, mid: 0, high: 0, bands: new Array(BAND_COUNT).fill(0) as number[], features: {} };
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
const timer = new THREE.Timer();
function animate() {
    requestAnimationFrame(animate);
    timer.update();

    // Compute audio data locally on every frame (no IPC)
    if (audioActive) {
        latestAudioData = computeAudioData();
    }
    if (currentModule && typeof currentModule.update === 'function') {
        const time = timer.getElapsed();

        // Proxy objects to allow sketches to modify FX parameters
        const bloomControl = {
            get strength() { return (bloomNode as any).strength.value; },
            set strength(v) { (bloomNode as any).strength.value = v; },
            get radius() { return (bloomNode as any).radius.value; },
            set radius(v) { (bloomNode as any).radius.value = v; },
            get threshold() { return (bloomNode as any).threshold.value; },
            set threshold(v) { (bloomNode as any).threshold.value = v; }
        };

        const rgbShiftControl = {
            get amount() { return (rgbNode as any).amount.value; },
            set amount(v) { 
                (rgbNode as any).amount.value = v; 
                rgbMix.value = v; 
            }
        };

        const filmControl = {
            get intensity() { return filmIntensityUniform.value; },
            set intensity(v) { 
                filmIntensityUniform.value = v; 
                filmMix.value = v; 
            }
        };

        const vignetteControl = {
            get offset() { return currentVignetteOffset; },
            set offset(v) { 
                currentVignetteOffset = v;
                vignetteOffsetUniform.value = v; 
            },
            get darkness() { return currentVignetteDarkness; },
            set darkness(v) { 
                currentVignetteDarkness = v;
                vignetteDarknessUniform.value = v; 
            }
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

    // Render using RenderPipeline
    postProcessing.render();

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
        strength: (bloomNode as any).strength.value,
        radius: (bloomNode as any).radius.value,
        threshold: (bloomNode as any).threshold.value,
        rgbAmount: (rgbNode as any).amount.value,
        filmIntensity: filmIntensityUniform.value,
        vignetteOffset: currentVignetteOffset,
        vignetteDarkness: currentVignetteDarkness
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
            bands: latestAudioData.bands || [],
            features: latestAudioData.features || {}
        }).catch(err => console.error("Audio activity emit error:", err));
    }

    lastSyncTime = now;
}

(async function() {
    await renderer.init();
    animate();
})();

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
