import * as THREE from 'three';
import { listen, emit } from '@tauri-apps/api/event';
import { convertFileSrc } from '@tauri-apps/api/core';

import Meyda from 'meyda';
import { WebGPURenderer, RenderPipeline, MeshStandardNodeMaterial, PointsNodeMaterial, LineBasicNodeMaterial, MeshBasicNodeMaterial, NodeMaterial } from 'three/webgpu';
import * as TSL from 'three/tsl';
import { bloom } from 'three/addons/tsl/display/BloomNode.js';
import { film } from 'three/addons/tsl/display/FilmNode.js';
import { rgbShift } from 'three/addons/tsl/display/RGBShiftNode.js';
import { CameraManager, type CameraStatus } from './cameraManager';
import { CameraNodeBindings } from './cameraNodeBindings';
import { clearScene } from './sceneCleanup';
import {
    CameraMotionAnalyzer,
    type CameraMotionConfig,
    type CameraMotionNodes
} from './cameraMotionAnalyzer';
import { GpuFeedbackService, type ShekereGpuApi } from './gpuFeedback';
import {
    analyzeFrequencyData,
    downsampleWaveform,
    normalizeWaveformChannels,
    type WaveformPreviewChannel
} from './audioAnalysis';
import { SketchLoader, type SketchLoadPayload } from './sketchLoader';
import {
    createFxRuntimePatch,
    createFxSettingsChangedPayload,
    haveFxRuntimeValuesChanged,
    shouldApplyFxSettings,
    type FxRuntimeValues,
    type FxSettingsChange
} from './fxSettings';
import { convertOscSketchData } from './oscData';

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

const cameraManager = new CameraManager({
    onStatus: (status: CameraStatus) => {
        emit('camera-status', status).catch(error => console.error('Camera status emit error:', error));
    },
    onDevices: (devices) => {
        emit('camera-device-list', { devices }).catch(error => console.error('Camera device list emit error:', error));
    }
});

listen<void>('request-camera-devices', () => { void cameraManager.refreshDevices(); });
listen<void>('request-camera-status', () => {
    emit('camera-status', cameraManager.getStatus()).catch(error => console.error('Camera status emit error:', error));
});
listen<{ deviceId: string }>('update-camera-device', (event) => {
    void cameraManager.selectDevice(event.payload.deviceId);
});
listen<void>('start-camera', () => { void cameraManager.start(); });
listen<void>('stop-camera', () => { cameraManager.stop(); });

const handleCameraDeviceChange = () => { void cameraManager.refreshDevices(); };
if (navigator.mediaDevices) {
    navigator.mediaDevices.addEventListener('devicechange', handleCameraDeviceChange);
}

window.addEventListener('beforeunload', () => {
    navigator.mediaDevices?.removeEventListener('devicechange', handleCameraDeviceChange);
    gpuFeedbackService.dispose();
    cameraMotionAnalyzer.dispose();
    cameraNodeBindings.dispose();
    cameraManager.dispose();
});



interface SketchConfig {
    audio?: {
        minFreqHz?: number;
        maxFreqHz?: number;
        features?: string[];
    };
    renderer?: {
        toneMapping?: THREE.ToneMapping;
        toneMappingExposure?: number;
    };
    camera?: {
        motion?: CameraMotionConfig;
    };
}

// --- 1. Three.js Basic Setup ---
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
camera.position.z = 5;

// WebGPURenderer is required for TSL
const renderer = new WebGPURenderer({ antialias: true });
const gpuFeedbackService = new GpuFeedbackService(renderer);
const cameraMotionAnalyzer = new CameraMotionAnalyzer(renderer, cameraManager.data.motion);
const cameraNodeBindings = new CameraNodeBindings();
const Shekere: {
    convertFileSrc: typeof convertFileSrc;
    clearScene: (container: THREE.Object3D) => void;
    SKETCH_DIR: string;
    camera: {
        textureNode: CameraNodeBindings['textureNode'];
        motion: CameraMotionNodes;
    };
    gpu: ShekereGpuApi;
} = {
    convertFileSrc,
    clearScene: (container: THREE.Object3D) => clearScene(container),
    SKETCH_DIR: "",
    camera: {
        textureNode: cameraNodeBindings.textureNode,
        motion: cameraMotionAnalyzer.nodes
    },
    gpu: gpuFeedbackService
};
(window as any).Shekere = Shekere;
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
currentPass = applyVignette(currentPass, vignetteOffsetUniform, vignetteDarknessUniform);

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
const WAVEFORM_PREVIEW_BUCKET_COUNT = 128;
const BASS_MAX_HZ = 250;
const MID_MAX_HZ = 2_000;
const DEFAULT_MIN_FREQ = 27.5;
const DEFAULT_MAX_FREQ = 4186;
const CORE_FEATURES = ['rms', 'zcr', 'energy', 'spectralCentroid', 'spectralFlatness', 'chroma', 'mfcc'];

interface AudioWaveform {
    mono: Float32Array;
    left: Float32Array;
    right: Float32Array;
}

interface AudioAnalysisData {
    volume: number;
    bass: number;
    mid: number;
    high: number;
    bands: number[];
    features: Record<string, unknown>;
    waveform: AudioWaveform;
}

// These buffers are part of the sketch API. Keep their identity stable so a
// render frame never allocates new full-resolution waveform arrays.
const waveform: AudioWaveform = {
    mono: new Float32Array(FFT_SIZE),
    left: new Float32Array(FFT_SIZE),
    right: new Float32Array(FFT_SIZE)
};

interface WaveformPreview {
    left: WaveformPreviewChannel;
    right: WaveformPreviewChannel;
}

const waveformPreview: WaveformPreview = {
    left: {
        min: new Array(WAVEFORM_PREVIEW_BUCKET_COUNT).fill(0),
        max: new Array(WAVEFORM_PREVIEW_BUCKET_COUNT).fill(0)
    },
    right: {
        min: new Array(WAVEFORM_PREVIEW_BUCKET_COUNT).fill(0),
        max: new Array(WAVEFORM_PREVIEW_BUCKET_COUNT).fill(0)
    }
};

let audioContext: AudioContext | null = null;
let analyserNode: AnalyserNode | null = null;
let channelSplitterNode: ChannelSplitterNode | null = null;
let leftWaveformAnalyserNode: AnalyserNode | null = null;
let rightWaveformAnalyserNode: AnalyserNode | null = null;
let audioSourceNode: MediaStreamAudioSourceNode | null = null;
let gainNode: GainNode | null = null;
let audioDataArray: Uint8Array | null = null;
let audioStream: MediaStream | null = null;
let audioActive = false;
let currentAudioSensitivity = 1.0;
let audioMinFreq = DEFAULT_MIN_FREQ;
let audioMaxFreq = DEFAULT_MAX_FREQ;
let currentAudioDeviceId: string | undefined = undefined;
let isMonoInput = false;
let reportedInputChannelCount: number | undefined = undefined;

let meydaAnalyzer: any = null;

function applyAudioConfig(config: { minFreqHz?: number; maxFreqHz?: number; features?: string[] }) {
    if (config.minFreqHz !== undefined) audioMinFreq = config.minFreqHz;
    if (config.maxFreqHz !== undefined) audioMaxFreq = config.maxFreqHz;
}

async function sendAudioDevices() {
    try {
        const devices = await navigator.mediaDevices.enumerateDevices();
        const audioInputs = devices.filter(d => d.kind === 'audioinput').map(d => ({
            deviceId: d.deviceId,
            label: d.label || `Microphone ${d.deviceId.slice(0, 5)}...`
        }));
        emit('audio-device-list', { devices: audioInputs }).catch(e => console.error(e));
    } catch (e) {
        console.error('Failed to enumerate devices:', e);
    }
}

listen('request-audio-devices', () => {
    sendAudioDevices();
});

listen<{ deviceId: string }>('update-audio-device', (event) => {
    currentAudioDeviceId = event.payload.deviceId;
    if (audioActive) {
        stopAudio();
        startAudio();
    }
});

if (navigator.mediaDevices) {
    navigator.mediaDevices.addEventListener('devicechange', sendAudioDevices);
}

async function startAudio() {
    if (audioActive) return;
    try {
        const audioConstraints = {
            autoGainControl: false,
            echoCancellation: false,
            noiseSuppression: false
        };
        const constraints = currentAudioDeviceId 
            ? { audio: { deviceId: { exact: currentAudioDeviceId }, ...audioConstraints }, video: false }
            : { audio: audioConstraints, video: false };
        audioStream = await navigator.mediaDevices.getUserMedia(constraints);
        sendAudioDevices();
        const AudioContextCtor = window.AudioContext || (window as any).webkitAudioContext;
        audioContext = new AudioContextCtor();
        analyserNode = audioContext.createAnalyser();
        analyserNode.fftSize = FFT_SIZE;
        analyserNode.smoothingTimeConstant = 0.5;
        analyserNode.minDecibels = -70;
        analyserNode.maxDecibels = -10;
        audioDataArray = new Uint8Array(new ArrayBuffer(analyserNode.frequencyBinCount));

        channelSplitterNode = audioContext.createChannelSplitter(2);
        leftWaveformAnalyserNode = audioContext.createAnalyser();
        rightWaveformAnalyserNode = audioContext.createAnalyser();
        leftWaveformAnalyserNode.fftSize = FFT_SIZE;
        rightWaveformAnalyserNode.fftSize = FFT_SIZE;
        
        gainNode = audioContext.createGain();
        gainNode.gain.value = currentAudioSensitivity;

        audioSourceNode = audioContext.createMediaStreamSource(audioStream);
        audioSourceNode.connect(gainNode);
        gainNode.connect(analyserNode);
        gainNode.connect(channelSplitterNode);
        channelSplitterNode.connect(leftWaveformAnalyserNode, 0);
        channelSplitterNode.connect(rightWaveformAnalyserNode, 1);

        // A mono source must still expose safe, equivalent left/right arrays.
        // WKWebView may omit channelCount for built-in Mac microphones, so the
        // waveform read also has an unknown-channel fallback below.
        reportedInputChannelCount = audioStream.getAudioTracks()[0]?.getSettings().channelCount;
        isMonoInput = reportedInputChannelCount === 1;
        
        // Always initialize Meyda with all core features
        meydaAnalyzer = Meyda.createMeydaAnalyzer({
            audioContext: audioContext,
            source: gainNode,
            bufferSize: FFT_SIZE,
            featureExtractors: CORE_FEATURES
        });
        
        audioActive = true;
    } catch (e) {
        console.error('Failed to start audio capture:', e);
    }
}

function stopAudio() {
    audioActive = false;
    audioSourceNode?.disconnect();
    gainNode?.disconnect();
    channelSplitterNode?.disconnect();
    leftWaveformAnalyserNode?.disconnect();
    rightWaveformAnalyserNode?.disconnect();
    if (audioStream) {
        audioStream.getTracks().forEach(t => t.stop());
        audioStream = null;
    }
    if (audioContext) {
        audioContext.close().catch(console.error);
        audioContext = null;
    }
    analyserNode = null;
    channelSplitterNode = null;
    leftWaveformAnalyserNode = null;
    rightWaveformAnalyserNode = null;
    audioSourceNode = null;
    gainNode = null;
    audioDataArray = null;
    meydaAnalyzer = null;
    isMonoInput = false;
    reportedInputChannelCount = undefined;
    waveform.mono.fill(0);
    waveform.left.fill(0);
    waveform.right.fill(0);
}

function readWaveform() {
    if (!analyserNode || !leftWaveformAnalyserNode || !rightWaveformAnalyserNode) {
        waveform.mono.fill(0);
        waveform.left.fill(0);
        waveform.right.fill(0);
        return;
    }

    analyserNode.getFloatTimeDomainData(waveform.mono);
    if (isMonoInput) {
        normalizeWaveformChannels(waveform, isMonoInput, reportedInputChannelCount);
        return;
    }

    leftWaveformAnalyserNode.getFloatTimeDomainData(waveform.left);
    rightWaveformAnalyserNode.getFloatTimeDomainData(waveform.right);

    // On macOS, a mono microphone can be exposed as a two-output splitter
    // while its MediaStreamTrack omits channelCount. Only use this fallback
    // when the device supplied no channel metadata; an explicitly stereo
    // device can therefore keep an intentionally silent right channel.
    normalizeWaveformChannels(waveform, isMonoInput, reportedInputChannelCount);
}

function updateWaveformPreview() {
    downsampleWaveform(waveform.left, waveformPreview.left);
    downsampleWaveform(waveform.right, waveformPreview.right);
}

function computeAudioData(): AudioAnalysisData {
    if (!analyserNode || !audioDataArray) {
        return {
            volume: 0,
            bass: 0,
            mid: 0,
            high: 0,
            bands: new Array(BAND_COUNT).fill(0) as number[],
            features: {},
            waveform
        };
    }

    analyserNode.getByteFrequencyData(audioDataArray as any);
    readWaveform();

    const sampleRate = audioContext?.sampleRate ?? 44100;
    const frequencySummary = analyzeFrequencyData(audioDataArray, sampleRate, {
        fftSize: FFT_SIZE,
        bandCount: BAND_COUNT,
        minFreq: audioMinFreq,
        maxFreq: audioMaxFreq,
        bassMaxHz: BASS_MAX_HZ,
        midMaxHz: MID_MAX_HZ
    });

    let features: Record<string, unknown> = {};
    if (meydaAnalyzer) {
        try {
            features = meydaAnalyzer.get(CORE_FEATURES) || {};
        } catch (e) {
            console.warn('Meyda feature extraction error:', e);
        }
    }

    return {
        ...frequencySummary,
        features,
        waveform
    };
}

// Listen for start/stop commands from the Control Panel
listen<void>('start-audio', () => { startAudio(); });
listen<void>('stop-audio', () => { stopAudio(); });

// Listen for Audio Sensitivity
listen<{ sensitivity: number }>('update-audio-sensitivity', (event) => {
    currentAudioSensitivity = event.payload.sensitivity;
    if (gainNode && audioContext) {
        gainNode.gain.setTargetAtTime(currentAudioSensitivity, audioContext.currentTime, 0.1);
    }
});

function applyFxSettings(changes: FxSettingsChange) {
    const patch = createFxRuntimePatch(changes);
    if (patch.strength !== undefined) (bloomNode as any).strength.value = patch.strength;
    if (patch.radius !== undefined) (bloomNode as any).radius.value = patch.radius;
    if (patch.threshold !== undefined) (bloomNode as any).threshold.value = patch.threshold;
    if (patch.rgbAmount !== undefined) {
        (rgbNode as any).amount.value = patch.rgbAmount;
        rgbMix.value = patch.rgbAmount;
    }
    if (patch.filmIntensity !== undefined) {
        filmIntensityUniform.value = patch.filmIntensity;
        filmMix.value = patch.filmIntensity;
    }
    if (patch.vignetteOffset !== undefined) {
        currentVignetteOffset = patch.vignetteOffset;
        vignetteOffsetUniform.value = patch.vignetteOffset;
    }
    if (patch.vignetteDarkness !== undefined) {
        currentVignetteDarkness = patch.vignetteDarkness;
        vignetteDarknessUniform.value = patch.vignetteDarkness;
    }
}

// Listen for Post-Processing settings from Control Panel. Ignore messages for a
// sketch that is no longer active so delayed cross-window events cannot leak FX.
listen<FxSettingsChange>('update-fx-settings', (event) => {
    if (!shouldApplyFxSettings(event.payload.sketchPath, sketchLoader.activeSketchPath)) return;
    applyFxSettings(event.payload);
});

// Legacy listener for backward compatibility during transition
listen<{ strength?: number; radius?: number; threshold?: number }>('update-bloom-settings', (event) => {
    const { strength, radius, threshold } = event.payload;
    if (strength !== undefined) (bloomNode as any).strength.value = strength;
    if (radius !== undefined) (bloomNode as any).radius.value = radius;
    if (threshold !== undefined) (bloomNode as any).threshold.value = threshold;
});

// --- 3. Shared state ---
let latestAudioData: AudioAnalysisData = {
    volume: 0,
    bass: 0,
    mid: 0,
    high: 0,
    bands: new Array(BAND_COUNT).fill(0) as number[],
    features: {},
    waveform
};
let latestMidiData = {
    notes: new Array(128).fill(0) as number[],
    cc: new Array(128).fill(0) as number[]
};
let latestOscData: Record<string, unknown> = {};
let oscEvents: { address: string; data: unknown }[] = [];

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

listen<{ address: string; args: unknown[] }>('osc-event', (event) => {
    const { address, args } = event.payload;
    const data = convertOscSketchData(address, args);
    latestOscData[address] = data;
    oscEvents.push({ address, data });
});

// --- 4. Render Loop ---
const timer = new THREE.Timer();
function animate() {
    requestAnimationFrame(animate);
    timer.update();
    const time = timer.getElapsed();

    // Compute audio data locally on every frame (no IPC)
    if (audioActive) {
        latestAudioData = computeAudioData();
    }
    cameraNodeBindings.update(cameraManager.data);
    cameraMotionAnalyzer.update(cameraManager.data);
    if (sketchLoader.currentModule) {
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
            camera: cameraManager.data,
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
            sketchLoader.currentModule.update(context);
        } catch (e) {
            console.error('Error in update:', e);
        }
        oscEvents.length = 0;
    }

    gpuFeedbackService.executeQueued(time);

    // Render using RenderPipeline
    postProcessing.render();

    // Send preview frame at low frequency
    emitPreviewFrame();

    // Throttled sync back to Control Panel
    syncToHost();
}

let lastSyncTime = 0;
let lastSyncedValues: FxRuntimeValues = {
    strength: -1, radius: -1, threshold: -1,
    rgbAmount: -1,
    filmIntensity: -1,
    vignetteOffset: -1, vignetteDarkness: -1
};
const SYNC_THROTTLE_MS = 100; // 10fps sync is enough for UI

function syncToHost() {
    const now = Date.now();
    if (now - lastSyncTime < SYNC_THROTTLE_MS) return;

    const currentValues: FxRuntimeValues = {
        strength: (bloomNode as any).strength.value,
        radius: (bloomNode as any).radius.value,
        threshold: (bloomNode as any).threshold.value,
        rgbAmount: (rgbNode as any).amount.value,
        filmIntensity: filmIntensityUniform.value,
        vignetteOffset: currentVignetteOffset,
        vignetteDarkness: currentVignetteDarkness
    };

    // Only emit if values have changed significantly
    const hasChanged = haveFxRuntimeValuesChanged(currentValues, lastSyncedValues);

    if (hasChanged) {
        emit('fx-settings-changed', createFxSettingsChangedPayload(
            sketchLoader.activeSketchPath,
            currentValues
        ));
        lastSyncedValues = { ...currentValues };
    }

    // Always emit audio activity if active (throttled to 10fps by this function)
    if (audioActive && latestAudioData) {
        updateWaveformPreview();
        emit('audio-activity', {
            volume: latestAudioData.volume || 0,
            bass: latestAudioData.bass || 0,
            mid: latestAudioData.mid || 0,
            high: latestAudioData.high || 0,
            bands: latestAudioData.bands || [],
            features: latestAudioData.features || {},
            waveformPreview
        }).catch(err => console.error("Audio activity emit error:", err));
    }

    lastSyncTime = now;
}

const rendererReady = (async function() {
    await renderer.init();
    animate();
    // Broadcast initial audio devices after a short delay to ensure host is listening
    setTimeout(sendAudioDevices, 500);
    setTimeout(() => { void cameraManager.refreshDevices(); }, 500);
})();

// --- 5. Dynamic Module Loader ---
const sketchLoader = new SketchLoader<THREE.Scene, SketchConfig, FxSettingsChange>({
    ready: rendererReady,
    scene,
    scope: gpuFeedbackService,
    createModuleUrl: (code) => URL.createObjectURL(new Blob([code], { type: 'application/javascript' })),
    revokeModuleUrl: (url) => URL.revokeObjectURL(url),
    importModule: (url) => import(/* @vite-ignore */ url),
    setSketchDirectory: (directory) => { Shekere.SKETCH_DIR = directory; },
    onSetupConfig: (config) => {
        // Default renderer state if not specified by sketch
        renderer.toneMapping = THREE.NoToneMapping;
        renderer.toneMappingExposure = 1.0;
        scene.background = null;

        if (config?.audio) applyAudioConfig(config.audio);
        if (config?.renderer) {
            if (config.renderer.toneMapping !== undefined) renderer.toneMapping = config.renderer.toneMapping;
            if (config.renderer.toneMappingExposure !== undefined) renderer.toneMappingExposure = config.renderer.toneMappingExposure;
        }
    },
    onModuleConfigured: (config) => cameraMotionAnalyzer.configure(config?.camera?.motion),
    applyFxSettings,
    onCleanupError: (error) => console.warn('Cleanup failed:', error),
    onLoadError: (error) => console.error('Failed to execute user sketch:', error),
    onUnexpectedError: (error) => console.error('Unexpected sketch loader error:', error)
});

listen<SketchLoadPayload<FxSettingsChange>>('user-code-update', (event) => {
    void sketchLoader.enqueue(event.payload);
});
