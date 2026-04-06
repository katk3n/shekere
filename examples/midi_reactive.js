/**
 * Simple MIDI Drum Pad Visualizer (Smoothed)
 * 
 * - Maps 16 MIDI pads (Notes 36-51) to a 4x4 grid of boxes.
 * - Includes Smoothing (Lerp) for MIDI CC transitions.
 */

const PADS_COUNT = 16;
const GRID_SIZE = 4;
const START_NOTE = 36;

let padMeshes = [];
let padStates = [];

// Current state for smoothing (lerp)
let currentRotationSpeed = 0;
let currentTilt = 0;
let currentGlobalScale = 1.0;

export function setup(scene) {
    const spacing = 1.5;
    const offset = (GRID_SIZE - 1) * spacing * 0.5;

    for (let i = 0; i < PADS_COUNT; i++) {
        // Grid position calculation
        const x = (i % GRID_SIZE) * spacing - offset;
        const y = Math.floor(i / GRID_SIZE) * spacing - offset;

        const geometry = new THREE.BoxGeometry(1, 1, 0.2);
        const material = new THREE.MeshStandardMaterial({
            color: new THREE.Color().setHSL(i / PADS_COUNT, 0.7, 0.3),
            emissive: new THREE.Color(0x000000)
        });

        const mesh = new THREE.Mesh(geometry, material);
        mesh.position.set(x, y, 0);
        scene.add(mesh);

        padMeshes.push(mesh);
        padStates.push(0);
    }

    // Lighting
    const light = new THREE.PointLight(0xffffff, 50);
    light.position.set(0, 0, 5);
    scene.add(light);
    scene.add(new THREE.AmbientLight(0x444444));
}

export function update(context) {
    const { midi } = context;

    // --- MIDI CC Acquisition & Smoothing ---
    // Use ?? instead of || to allow zero values, and lerp for smooth transitions.
    
    // CC 1: Rotation Speed (Default: 0.1)
    const targetRotationSpeed = (midi.cc[1] ?? 0.1) * 0.1;
    currentRotationSpeed += (targetRotationSpeed - currentRotationSpeed) * 0.1;

    // CC 10: Horizontal Tilt (Default: 0.5 = center)
    const targetTilt = ((midi.cc[10] ?? 0.5) - 0.5) * 2.0;
    currentTilt += (targetTilt - currentTilt) * 0.1;

    // CC 11: Global Scale (Default: 0.5 -> overall scale 1.25)
    const targetGlobalScale = 0.5 + (midi.cc[11] ?? 0.5) * 1.5;
    currentGlobalScale += (targetGlobalScale - currentGlobalScale) * 0.1;

    // --- Update Individual Pads ---
    for (let i = 0; i < PADS_COUNT; i++) {
        const noteNumber = START_NOTE + i;
        const velocity = midi.notes[noteNumber] || 0;

        if (velocity > 0) {
            padStates[i] = velocity;
        }

        const mesh = padMeshes[i];
        const state = padStates[i];

        // Scale (Smoothed globalScale + hit reaction)
        const s = currentGlobalScale + state * 1.0;
        mesh.scale.set(s, s, 1);

        // Emissive light
        mesh.material.emissive.setHSL(i / PADS_COUNT, 0.8, state * 0.6);
        
        // Rotation (Using smoothed values)
        mesh.rotation.z += currentRotationSpeed + state * 0.1;
        mesh.rotation.y = currentTilt;

        // Decay
        padStates[i] *= 0.92;
    }
}

export function cleanup(scene) {
    padMeshes.forEach(mesh => {
        scene.remove(mesh);
        mesh.geometry.dispose();
        mesh.material.dispose();
    });
    padMeshes = [];
    padStates = [];
}
