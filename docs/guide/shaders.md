# Shaders (TSL & WebGPU)

Shekere supports the **Three.js Shading Language (TSL)** and **WebGPU** to achieve high-performance procedural graphics and effects.

TSL provides a unified, JavaScript-first shader authoring experience.

## What is TSL?

TSL (Three.js Shading Language) allows you to construct shaders entirely in JavaScript/TypeScript using a node-based architecture. Instead of writing GLSL strings, you compose mathematical operations and graphics logic using JavaScript functions.

These JavaScript node structures are then automatically compiled by Three.js into highly optimized WGSL (WebGPU Shading Language) at runtime.

### Why TSL?

1. **Unified Language**: Write your CPU logic and GPU shader logic in the exact same language (JavaScript).
2. **Reusability**: Shader snippets are just JavaScript functions. You can import, export, and compose them infinitely.
3. **WebGPU Native**: TSL is built from the ground up for the modern WebGPU pipeline, enabling compute shaders and massive performance gains.

## Accessing TSL in Shekere

In Shekere, the global `TSL` object is automatically available in all your sketches. You do not need to import it.

```javascript
export function setup(scene) {
    // Basic example of using TSL
    const myColorNode = TSL.vec3(1.0, 0.0, 0.0); // A pure red color node
    
    const material = new THREE.MeshBasicNodeMaterial();
    material.colorNode = myColorNode;

    const mesh = new THREE.Mesh(new THREE.BoxGeometry(), material);
    scene.add(mesh);
}
```

## Custom Functions (Fn)

For complex math, chaining methods like `.mul()`, `.add()`, and `.sub()` can get difficult to read. TSL provides the `Fn()` wrapper, which allows you to write JavaScript logic that gets translated to shader code!

```javascript
const calculateGlow = TSL.Fn(() => {
    // TSL.uv() gets the UV coordinates of the current fragment
    const d = TSL.distance(TSL.uv(), TSL.vec2(0.5));
    
    // Smoothstep creates a soft circular gradient
    let strength = TSL.smoothstep(0.5, 0.2, d);
    
    return strength;
});

const material = new THREE.MeshBasicNodeMaterial({ transparent: true });
material.opacityNode = calculateGlow();
material.colorNode = TSL.vec3(0.0, 1.0, 1.0); // Cyan
```

## Particle Systems & InstancedMesh

**Important Note for WebGPU:** The standard `THREE.Points` and `PointsMaterial` are severely restricted under WebGPU. WebGPU specifications restrict point sizes to exactly `1.0` pixel, meaning you cannot create large, glowing particles using standard points.

To create scalable particle systems, **you must use `THREE.InstancedMesh`** combined with custom vertex billboarding in TSL.

### Example: Scalable Particles via InstancedMesh

Instead of `THREE.Points`, use a small `PlaneGeometry` instanced thousands of times. By utilizing TSL's `vertexNode`, we can make the planes always face the camera (billboarding).

```javascript
export function setup(scene) {
    const COUNT = 1000;
    const geometry = new THREE.PlaneGeometry(1, 1);
    
    // Pass custom attributes for each instance
    const positions = new Float32Array(COUNT * 3);
    for(let i = 0; i < COUNT * 3; i++) positions[i] = (Math.random() - 0.5) * 10;
    geometry.setAttribute('instanceOffset', new THREE.InstancedBufferAttribute(positions, 3));

    const material = new THREE.MeshBasicNodeMaterial();

    // 1. Read the custom attribute
    const instanceOffset = TSL.attribute('instanceOffset', 'vec3');
    
    // 2. Calculate Billboarding (always face camera)
    const viewOffset = TSL.cameraViewMatrix.mul(TSL.modelWorldMatrix).mul(TSL.vec4(instanceOffset, 1.0));
    const finalViewPos = viewOffset.add(TSL.vec4(TSL.positionLocal.xy, 0.0, 0.0));
    
    // 3. Override the default vertex transform
    material.vertexNode = TSL.cameraProjectionMatrix.mul(finalViewPos);

    const instancedMesh = new THREE.InstancedMesh(geometry, material, COUNT);
    // Disable culling because we are manually moving vertices in the shader
    instancedMesh.frustumCulled = false; 
    
    scene.add(instancedMesh);
}
```

Check out the `shader_stars.js` example in the repository to see an advanced implementation of this technique reacting to MIDI input!
