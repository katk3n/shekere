import * as THREE from 'three';
import { listen, emit } from '@tauri-apps/api/event';

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

let currentModule: SketchModule | null = null;
let latestAudioData = { volume: 0, bass: 0, mid: 0, high: 0, bands: new Array(256).fill(0) as number[] };
let latestMidiData = {
    notes: new Array(128).fill(0) as number[],
    cc: new Array(128).fill(0) as number[]
};
let latestOscData: Record<string, any> = {};
let oscEvents: { address: string; data: any }[] = [];

listen<{ volume: number; bass: number; mid: number; high: number; bands: number[] }>('audio-data', (event) => {
    latestAudioData = event.payload;
});

listen<{ status: number; data1: number; data2: number }>('midi-event', (event) => {
    const { status, data1, data2 } = event.payload;
    const type = status & 0xF0;
    
    if (type === 0x90) { // Note On
        latestMidiData.notes[data1] = data2 / 127.0;
    } else if (type === 0x80) { // Note Off
        latestMidiData.notes[data1] = 0;
    } else if (type === 0xB0) { // CC
        latestMidiData.cc[data1] = data2 / 127.0;
    }
});

listen<{ address: string; args: any[] }>('osc-event', (event) => {
    const { address, args } = event.payload;
    
    let data: any = args;
    // Automatically convert key-value pairs into an object if it's /dirt/play or similar
    // tidal (superdirt) sends [key, val, key, val, ...]
    if (address === '/dirt/play' && args.length % 2 === 0) {
        const obj: Record<string, any> = {};
        for (let i = 0; i < args.length; i += 2) {
            const key = String(args[i]);
            obj[key] = args[i+1];
        }
        data = obj;
    }
    
    latestOscData[address] = data;
    oscEvents.push({ address, data });
});

// --- 2. Render Loop ---
const clock = new THREE.Clock();
function animate() {
    requestAnimationFrame(animate);
    
    // Call user's update function if it exists
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
            console.error("Error in update:", e);
        }
        
        // Clear events after the frame has processed them
        oscEvents.length = 0;
    }
    
    renderer.render(scene, camera);
}
animate();

// --- 3. Dynamic Module Loader ---
listen<{ code: string }>('user-code-update', async (event) => {
    try {
        const jsCode = event.payload.code;
        // Convert the raw string into a Blob URL representing a JS module
        const blob = new Blob([jsCode], { type: 'application/javascript' });
        const blobUrl = URL.createObjectURL(blob);
        
        // Cleanup old module objects from the scene to prevent memory leaks
        if (currentModule && typeof currentModule.cleanup === 'function') {
            try {
                currentModule.cleanup(scene);
            } catch (e) {
                console.warn("Cleanup failed:", e);
            }
        }
        
        // Dynamically import the user module
        const userModule = await import(/* @vite-ignore */ blobUrl);
        
        // To use 'this' like 'this.mesh = ...' as per ADR,
        // bind and call using an independent State object as 'this'
        const sketchContext = {};
        
        // Run setup and add meshes to the scene
        if (typeof userModule.setup === 'function') {
            const config = userModule.setup.call(sketchContext, scene);
            if (config && config.audio) {
                emit('audio-config-update', config.audio);
            }
        }
        
        // Create an interface object for the per-frame loop
        currentModule = {
            update: (ctx: any) => userModule.update?.call(sketchContext, ctx),
            cleanup: (s: any) => userModule.cleanup?.call(sketchContext, s)
        };
        
        console.log("Successfully hot-reloaded user code.");
        
        // Cleanup the object URL
        URL.revokeObjectURL(blobUrl);
    } catch (e: any) {
        console.error("Failed to execute user sketch:", e);
    }
});
