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

const renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true });
renderer.setSize(window.innerWidth, window.innerHeight);
// ADR: 背景透過設定（Visualizerは透明を想定）
renderer.setClearColor(0x000000, 0);
document.body.appendChild(renderer.domElement);

window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
});

let currentModule: SketchModule | null = null;

// --- 2. Render Loop ---
const clock = new THREE.Clock();
function animate() {
    requestAnimationFrame(animate);
    
    // Call user's update function if it exists
    if (currentModule && typeof currentModule.update === 'function') {
        const time = clock.getElapsedTime();
        // 開発フェーズ3でaudioやmidiデータが追加される想定
        const context = { time }; 
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
