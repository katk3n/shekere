/**
 * audio_reactive_knot.js - Audio-Reactive TorusKnot
 */

export function setup(scene) {
    const geometry = new THREE.TorusKnotGeometry(2, 0.4, 128, 32);
    const material = new THREE.MeshStandardMaterial({
        color: 0x4400ff,
        emissive: 0x4400ff,
        emissiveIntensity: 0.5
    });
    this.mesh = new THREE.Mesh(geometry, material);
    scene.add(this.mesh);

    const light = new THREE.PointLight(0xffffff, 50);
    light.position.set(5, 5, 5);
    scene.add(light);
    this.light = light;

    return {
        audio: { minFreqHz: 20, maxFreqHz: 2000 }
    };
}

export function update(context) {
    const { time, audio, bloom } = context;

    this.mesh.rotation.y = time * 0.5;
    this.mesh.rotation.x = time * 0.2;

    const scale = 1.0 + audio.bass * 1.5;
    this.mesh.scale.set(scale, scale, scale);
    
    // Bloom reactivity (Dynamic)
    bloom.strength = audio.bass * 4.0;
    
    // Set defaults once if not initialized
    if (!this.fxInitialized) {
        bloom.radius = 0.5;
        bloom.threshold = 0.1;
        this.fxInitialized = true;
    }
}

export function cleanup(scene) {
    Shekere.clearScene(scene);
}
