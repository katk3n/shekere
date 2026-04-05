import * as THREE from 'three';
import { listen } from '@tauri-apps/api/event';

// Expose THREE globally so user sketches can use it without importing
(window as any).THREE = THREE;

// ユーザーが提供するスケッチモジュールの型定義
interface SketchModule {
    setup?: (scene: THREE.Scene) => void;
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
            midi: latestMidiData 
        }; 
        currentModule.update(context);
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
            currentModule.cleanup(scene);
        }
        
        // Dynamically import the user module
        const userModule = await import(/* @vite-ignore */ blobUrl);
        
        // ADRで 'this.mesh = ...' のように this を利用するため、
        // 独立した State オブジェクトを this として bind して呼び出す
        const sketchContext = {};
        
        // Run setup and add meshes to the scene
        if (typeof userModule.setup === 'function') {
            userModule.setup.call(sketchContext, scene);
        }
        
        // 毎フレームのループ用インターフェースオブジェクトを作成
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
