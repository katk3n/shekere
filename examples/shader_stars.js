/**
 * MIDI Glowing Starfield (Infinite Resolution Procedural Version)
 * 
 * - Each MIDI note (0-127) triggers specific groups of stars.
 * - Procedural shader-based glow for perfectly smooth circles.
 */

const STAR_COUNT = 1500;
let points;
let starData = [];

// Vertex shader for drawing procedural (mathematical) circles
const vertexShader = `
  attribute float size;
  attribute vec3 customColor;
  varying vec3 vColor;
  void main() {
    vColor = customColor;
    vec4 mvPosition = modelViewMatrix * vec4(position, 1.0);
    // Distance-based size calculation
    gl_PointSize = size * (200.0 / -mvPosition.z);
    gl_Position = projectionMatrix * mvPosition;
  }
`;

// Fragment shader for drawing perfectly smooth spheres of light
const fragmentShader = `
  varying vec3 vColor;
  void main() {
    // Get distance from center in range 0.0 to 0.5
    float d = distance(gl_PointCoord, vec2(0.5));
    // Smooth attenuation using exponential function (No pixelation as it doesn't use textures)
    float strength = exp(-d * 8.0);
    // Soften the edges further
    strength *= (1.0 - smoothstep(0.4, 0.5, d));
    
    gl_FragColor = vec4(vColor, strength);
  }
`;

export function setup(scene) {
    const positions = new Float32Array(STAR_COUNT * 3);
    const colors = new Float32Array(STAR_COUNT * 3);
    const sizes = new Float32Array(STAR_COUNT);

    const colorObj = new THREE.Color();

    for (let i = 0; i < STAR_COUNT; i++) {
        const r = 15 + Math.random() * 35;
        const theta = Math.random() * Math.PI * 2;
        const phi = Math.acos(2 * Math.random() - 1);

        positions[i * 3] = r * Math.sin(phi) * Math.cos(theta);
        positions[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
        positions[i * 3 + 2] = r * Math.cos(phi);

        // Vibrant colors
        colorObj.setHSL(Math.random(), 0.9, 0.6);
        colors[i * 3] = colorObj.r;
        colors[i * 3 + 1] = colorObj.g;
        colors[i * 3 + 2] = colorObj.b;

        sizes[i] = 1.0;

        starData.push({
            noteIndex: i % 128,
            baseSize: 0.5 + Math.random() * 1.0,
            twinkleSpeed: 0.5 + Math.random() * 1.5,
            phase: Math.random() * Math.PI * 2,
            r: colorObj.r,
            g: colorObj.g,
            b: colorObj.b
        });
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute('customColor', new THREE.BufferAttribute(colors, 3));
    geometry.setAttribute('size', new THREE.BufferAttribute(sizes, 1));

    const material = new THREE.ShaderMaterial({
        vertexShader,
        fragmentShader,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
        transparent: true
    });

    points = new THREE.Points(geometry, material);
    scene.add(points);

    const ambientLight = new THREE.AmbientLight(0x222222);
    scene.add(ambientLight);
    this.ambientLight = ambientLight;
}

export function update(context) {
    const { time, midi } = context;
    if (!points) return;

    const sizes = points.geometry.attributes.size.array;
    const colors = points.geometry.attributes.customColor.array;
    const bgScale = midi.cc[7] || 0.2;

    for (let i = 0; i < STAR_COUNT; i++) {
        const data = starData[i];
        const vel = midi.notes[data.noteIndex] || 0;

        // Velocity reaction (Smooth size variation)
        const hitEffect = Math.pow(vel, 0.5) * 12.0;
        const twinkle = Math.sin(time * data.twinkleSpeed + data.phase) * 0.1 + 0.9;

        sizes[i] = data.baseSize * (twinkle + bgScale * 2.0 + hitEffect);

        // Color maintenance (Avoid white-out, keep intensity around 1.0)
        const intensity = 1.0 + Math.min(vel, 1.0) * 0.2;
        colors[i * 3] = data.r * intensity;
        colors[i * 3 + 1] = data.g * intensity;
        colors[i * 3 + 2] = data.b * intensity;
    }

    points.geometry.attributes.size.needsUpdate = true;
    points.geometry.attributes.customColor.needsUpdate = true;

    const tilt = (midi.cc[10] || 0.5) - 0.5;
    points.rotation.y += 0.003 + tilt * 0.05;
    points.rotation.z += 0.001;
}

export function cleanup(scene) {
  clearScene(scene);
}
