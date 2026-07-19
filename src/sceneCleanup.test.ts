import { describe, expect, it, vi } from "vitest";
import * as THREE from "three";
import { clearScene } from "./sceneCleanup";

describe("clearScene", () => {
  it("clears and disposes Mesh, Line, Points, and Sprite resources", () => {
    const scene = new THREE.Scene();
    const group = new THREE.Group();
    const meshGeometry = new THREE.BoxGeometry();
    const lineGeometry = new THREE.BufferGeometry();
    const pointsGeometry = new THREE.BufferGeometry();
    const meshMaterial = new THREE.MeshBasicMaterial();
    const lineMaterial = new THREE.LineBasicMaterial();
    const pointsMaterial = new THREE.PointsMaterial();
    const spriteMaterial = new THREE.SpriteMaterial();
    const resources = [
      meshGeometry,
      lineGeometry,
      pointsGeometry,
      meshMaterial,
      lineMaterial,
      pointsMaterial,
      spriteMaterial,
    ];
    const disposeSpies = resources.map((resource) => vi.spyOn(resource, "dispose"));

    group.add(
      new THREE.Mesh(meshGeometry, meshMaterial),
      new THREE.Line(lineGeometry, lineMaterial),
      new THREE.Points(pointsGeometry, pointsMaterial),
      new THREE.Sprite(spriteMaterial),
    );
    scene.add(group);

    clearScene(scene);

    expect(scene.children).toHaveLength(0);
    disposeSpies.forEach((dispose) => expect(dispose).toHaveBeenCalledOnce());
  });

  it("disposes shared geometry and materials exactly once", () => {
    const scene = new THREE.Scene();
    const geometry = new THREE.PlaneGeometry();
    const material = new THREE.MeshBasicMaterial();
    const geometryDispose = vi.spyOn(geometry, "dispose");
    const materialDispose = vi.spyOn(material, "dispose");

    scene.add(
      new THREE.Mesh(geometry, material),
      new THREE.Mesh(geometry, material),
      new THREE.Mesh(geometry, [material, material]),
    );

    clearScene(scene);

    expect(geometryDispose).toHaveBeenCalledOnce();
    expect(materialDispose).toHaveBeenCalledOnce();
  });

  it("does not dispose textures referenced by materials", () => {
    const scene = new THREE.Scene();
    const texture = new THREE.Texture();
    const textureDispose = vi.spyOn(texture, "dispose");
    const material = new THREE.MeshBasicMaterial({ map: texture });
    scene.add(new THREE.Mesh(new THREE.PlaneGeometry(), material));

    clearScene(scene);

    expect(textureDispose).not.toHaveBeenCalled();
  });

  it("is a no-op when the container is already empty", () => {
    const scene = new THREE.Scene();
    expect(() => clearScene(scene)).not.toThrow();
    expect(scene.children).toHaveLength(0);
  });
});
