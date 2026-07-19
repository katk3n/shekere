import * as THREE from "three";

type RenderableObject = THREE.Object3D & {
  geometry?: THREE.BufferGeometry;
  material?: THREE.Material | THREE.Material[];
};

/**
 * Removes all descendants and disposes scene-owned geometry and material
 * resources exactly once. Textures and other indirectly referenced resources
 * are intentionally excluded because their ownership cannot be inferred.
 */
export function clearScene(container: THREE.Object3D): void {
  const geometries = new Set<THREE.BufferGeometry>();
  const materials = new Set<THREE.Material>();

  for (const child of container.children) {
    child.traverse((object) => {
      const renderable = object as RenderableObject;
      if (renderable.geometry) geometries.add(renderable.geometry);

      if (Array.isArray(renderable.material)) {
        renderable.material.forEach((material) => materials.add(material));
      } else if (renderable.material) {
        materials.add(renderable.material);
      }
    });
  }

  container.clear();
  geometries.forEach((geometry) => geometry.dispose());
  materials.forEach((material) => material.dispose());
}
