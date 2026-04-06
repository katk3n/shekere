// examples/tidal_reactive.js
// Sketch example reflecting TidalCycles (SuperDirt) OSC (/dirt/play)
// Digital Grid & Shockwave version

let mesh; // Central Sphere (Kick)
let shockwave; // Expanding Ring (Snare)
let gridGroup; // Digital Grid (Hat)
const gridCubes = [];

// State management for hit decay
const state = {
    kick: 0,
    snare: 0,
    hat: 0
};

export function setup(scene) {
    // --- 1. Kick (Central Icosahedron) ---
    const geometry = new THREE.IcosahedronGeometry(1, 2);
    const material = new THREE.MeshPhongMaterial({ 
        color: 0x00ffff, 
        wireframe: true,
        emissive: 0x004444
    });
    mesh = new THREE.Mesh(geometry, material);
    scene.add(mesh);

    // --- 2. Snare (Expanding Shockwave) ---
    const torusGeo = new THREE.TorusGeometry(4, 0.05, 16, 100);
    const torusMat = new THREE.MeshBasicMaterial({ 
        color: 0xff00ff, 
        transparent: true, 
        opacity: 0 
    });
    shockwave = new THREE.Mesh(torusGeo, torusMat);
    shockwave.rotation.x = Math.PI / 2;
    scene.add(shockwave);

    // --- 3. Hat (Digital Grid of Cubes) ---
    gridGroup = new THREE.Group();
    const boxGeo = new THREE.BoxGeometry(0.1, 0.1, 0.1);
    const boxMat = new THREE.MeshLambertMaterial({ color: 0xaaaaaa });
    
    const count = 5;
    const spacing = 4;
    for (let x = -count; x <= count; x++) {
        for (let y = -count; y <= count; y++) {
            const cube = new THREE.Mesh(boxGeo, boxMat.clone());
            cube.position.set(x * spacing, y * spacing, -10);
            gridGroup.add(cube);
            gridCubes.push({
                mesh: cube,
                origX: x * spacing,
                origY: y * spacing
            });
        }
    }
    scene.add(gridGroup);

    // --- 4. Lights ---
    const light = new THREE.PointLight(0xffffff, 50);
    light.position.set(5, 5, 5);
    scene.add(light);
    scene.add(new THREE.AmbientLight(0x222222));
}

export function update(context) {
    const { time, oscEvents } = context;
    if (!mesh || !shockwave || !gridGroup) return;

    // --- 1. OSC Event Processing ---
    for (const event of oscEvents) {
        if (event.address === '/dirt/play') {
            const play = event.data;
            const sample = play.s;
            if (sample === 'bd') state.kick = 1.0;
            if (sample === 'sd') state.snare = 1.0;
            if (sample === 'hc' || sample === 'hh' || sample === 'sh') state.hat = 1.0;
        }
    }

    // --- 2. Visual Updates ---
    
    // Kick (Center): Pulse scale and rotation
    const kScale = 1.0 + state.kick * 2.0;
    mesh.scale.set(kScale, kScale, kScale);
    mesh.rotation.y += 0.01 + state.kick * 0.2;
    mesh.material.emissiveIntensity = state.kick * 2;

    // Snare (Shockwave): Expand and fade
    if (state.snare > 0.01) {
        shockwave.visible = true;
        const sScale = 1.0 + (1.0 - state.snare) * 4.0;
        shockwave.scale.set(sScale, sScale, sScale);
        shockwave.material.opacity = state.snare * 0.8;
    } else {
        shockwave.visible = false;
    }

    // Hat (Digital Grid): Glitch jitter
    gridCubes.forEach((cube, i) => {
        const jitter = state.hat * 0.5;
        cube.mesh.position.x = cube.origX + (Math.random() - 0.5) * jitter;
        cube.mesh.position.y = cube.origY + (Math.random() - 0.5) * jitter;
        
        // Cubes flow towards the camera slightly
        cube.mesh.position.z = -10 + Math.sin(time + i) * 2;
        
        // Color shift on hit
        cube.mesh.material.emissive.setHSL(0.6, 1, state.hat * 0.5);
    });
    gridGroup.rotation.z = time * 0.1;

    // --- 3. Decay ---
    state.kick *= 0.9;
    state.snare *= 0.85;
    state.hat *= 0.8;
}

export function cleanup(scene) {
    if (mesh) {
        scene.remove(mesh);
        mesh.geometry.dispose();
        mesh.material.dispose();
    }
    if (shockwave) {
        scene.remove(shockwave);
        shockwave.geometry.dispose();
        shockwave.material.dispose();
    }
    if (gridGroup) {
        scene.remove(gridGroup);
        gridCubes.forEach(c => {
            c.mesh.geometry.dispose();
            c.mesh.material.dispose();
        });
    }
}
