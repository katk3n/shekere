// examples/effects_showcase.js
// Phase 4.5 Showcase: Demonstrates all post-processing effects.

export function setup(scene) {
    // 1. Add a background wall so Vignette and Film Grain are actually visible
    // (Vignette on a pure black background does nothing)
    const bgGeometry = new THREE.PlaneGeometry(30, 30);
    const bgMaterial = new THREE.MeshStandardMaterial({ color: 0x222233 });
    this.bgMesh = new THREE.Mesh(bgGeometry, bgMaterial);
    this.bgMesh.position.z = -10;
    scene.add(this.bgMesh);

    // 2. Use a solid, colorful shape so RGBShift has pixels to distort 
    const geometry = new THREE.TorusKnotGeometry(1.5, 0.4, 128, 32);
    const material = new THREE.MeshStandardMaterial({
        color: 0x00ffcc,
        emissive: 0x00ffcc,
        emissiveIntensity: 0.5,
        wireframe: false // Solid looks better for bloom and shift
    });
    this.mesh = new THREE.Mesh(geometry, material);
    scene.add(this.mesh);

    // 3. Add some floating cubes for more visual noise
    this.cubes = [];
    for (let i = 0; i < 5; i++) {
        const cube = new THREE.Mesh(
            new THREE.BoxGeometry(0.5, 0.5, 0.5),
            new THREE.MeshStandardMaterial({ color: 0xff0055, emissive: 0xff0055 })
        );
        cube.position.set((Math.random() - 0.5) * 8, (Math.random() - 0.5) * 8, -5 + Math.random() * 5);
        scene.add(cube);
        this.cubes.push(cube);
    }

    const light = new THREE.PointLight(0xffffff, 50);
    light.position.set(0, 0, 5);
    scene.add(light);
    
    // Ambient light so the background is visible
    scene.add(new THREE.AmbientLight(0x404040));

    // Set initial baseline for effects
    return {
        audio: { minFreqHz: 20, maxFreqHz: 2000 }
    };
}

export function update(context) {
    const { time, audio, bloom, rgbShift, film, vignette } = context;

    // Spin the knot
    this.mesh.rotation.y = time * 0.5;
    this.mesh.rotation.x = time * 0.2;

    // Spin the cubes
    this.cubes.forEach((cube, i) => {
        cube.rotation.x += 0.01 * (i + 1);
        cube.rotation.y += 0.02;
    });

    // --- Meshes Reactivity ---
    // Boost material emissive on bass
    this.mesh.material.emissiveIntensity = 0.5 + audio.bass * 3.0;
    
    // Scale on mid-range audio
    const scale = 1.0 + audio.mid * 1.5;
    this.mesh.scale.set(scale, scale, scale);

    // * UI Sliders for Bloom, RGB Shift, Film Grain, and Vignette can now be 
    // played with manually from the Control Panel for VJing! *
}

export function cleanup(scene) {
    [this.mesh, this.bgMesh, ...this.cubes].forEach(obj => {
        if (!obj) return;
        scene.remove(obj);
        if (obj.geometry) obj.geometry.dispose();
        if (obj.material) obj.material.dispose();
    });
}
