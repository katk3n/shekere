import { describe, expect, it, vi } from "vitest";
import {
  SketchLoader,
  type ImportedSketchModule,
  type SketchLoaderDependencies,
} from "./sketchLoader";

interface Scene {
  name: string;
}

interface Config {
  mode: string;
}

interface FxSettings {
  strength: number;
}

function createHarness(modules: Array<ImportedSketchModule<Scene, Config>>) {
  const scene = { name: "scene" };
  const scope = {
    disposeActiveScope: vi.fn(),
    beginCandidateScope: vi.fn(),
    commitCandidateScope: vi.fn(),
    rollbackCandidateScope: vi.fn(),
  };
  const createModuleUrl = vi.fn((code: string) => `blob:${code}`);
  const revokeModuleUrl = vi.fn();
  const importModule = vi.fn(async () => {
    const module = modules.shift();
    if (!module) throw new Error("No module queued");
    return module;
  });
  const dependencies: SketchLoaderDependencies<Scene, Config, FxSettings> = {
    ready: Promise.resolve(),
    scene,
    scope,
    createModuleUrl,
    revokeModuleUrl,
    importModule,
    setSketchDirectory: vi.fn(),
    onSetupConfig: vi.fn(),
    onModuleConfigured: vi.fn(),
    applyFxSettings: vi.fn(),
    onCleanupError: vi.fn(),
    onLoadError: vi.fn(),
    onUnexpectedError: vi.fn(),
  };

  return {
    loader: new SketchLoader(dependencies),
    dependencies,
    scene,
    scope,
    createModuleUrl,
    revokeModuleUrl,
    importModule,
  };
}

describe("SketchLoader", () => {
  it("loads, configures, activates, and revokes a sketch module", async () => {
    let setupContext: Record<string, unknown> | undefined;
    let updateContext: Record<string, unknown> | undefined;
    const update = vi.fn(function (this: Record<string, unknown>) {
      updateContext = this;
    });
    const config = { mode: "custom" };
    const harness = createHarness([{
      setup(this: Record<string, unknown>, scene) {
        setupContext = this;
        expect(scene).toBe(harness.scene);
        return config;
      },
      update,
    }]);

    await harness.loader.load({
      code: "first",
      dir: "/sketches/",
      sketchPath: "/sketches/first.js",
      fxSettings: { strength: 0.5 },
    });
    harness.loader.currentModule?.update({ frame: 1 });

    expect(harness.dependencies.setSketchDirectory).toHaveBeenCalledWith("/sketches/");
    expect(harness.dependencies.onSetupConfig).toHaveBeenCalledWith(config);
    expect(harness.dependencies.onModuleConfigured).toHaveBeenCalledWith(config);
    expect(harness.scope.commitCandidateScope).toHaveBeenCalledOnce();
    expect(harness.loader.activeSketchPath).toBe("/sketches/first.js");
    expect(harness.dependencies.applyFxSettings).toHaveBeenCalledWith({ strength: 0.5 });
    expect(update).toHaveBeenCalledWith({ frame: 1 });
    expect(updateContext).toBe(setupContext);
    expect(harness.revokeModuleUrl).toHaveBeenCalledWith("blob:first");
  });

  it("cleans up the previous module with the same sketch context", async () => {
    let setupContext: Record<string, unknown> | undefined;
    let cleanupContext: Record<string, unknown> | undefined;
    const cleanup = vi.fn(function (this: Record<string, unknown>) {
      cleanupContext = this;
    });
    const harness = createHarness([
      {
        setup(this: Record<string, unknown>) {
          setupContext = this;
          return { mode: "first" };
        },
        cleanup,
      },
      { setup: () => ({ mode: "second" }) },
    ]);

    await harness.loader.load({ code: "first" });
    await harness.loader.load({ code: "second" });

    expect(cleanup).toHaveBeenCalledWith(harness.scene);
    expect(cleanupContext).toBe(setupContext);
    expect(harness.scope.disposeActiveScope).toHaveBeenCalledTimes(2);
    expect(harness.loader.currentModule).not.toBeNull();
  });

  it("continues loading after cleanup throws", async () => {
    const cleanupError = new Error("cleanup failed");
    const harness = createHarness([
      { cleanup: () => { throw cleanupError; } },
      { setup: () => ({ mode: "replacement" }) },
    ]);

    await harness.loader.load({ code: "first" });
    await harness.loader.load({ code: "second", sketchPath: "second.js" });

    expect(harness.dependencies.onCleanupError).toHaveBeenCalledWith(cleanupError);
    expect(harness.scope.disposeActiveScope).toHaveBeenCalledTimes(2);
    expect(harness.scope.commitCandidateScope).toHaveBeenCalledTimes(2);
    expect(harness.loader.activeSketchPath).toBe("second.js");
  });

  it("rolls back failed setup and always revokes its module URL", async () => {
    const setupError = new Error("setup failed");
    const harness = createHarness([{
      setup: () => { throw setupError; },
    }]);

    await harness.loader.load({ code: "broken", sketchPath: "broken.js" });

    expect(harness.scope.rollbackCandidateScope).toHaveBeenCalledOnce();
    expect(harness.scope.commitCandidateScope).not.toHaveBeenCalled();
    expect(harness.dependencies.onModuleConfigured).toHaveBeenCalledWith(undefined);
    expect(harness.dependencies.onLoadError).toHaveBeenCalledWith(setupError);
    expect(harness.loader.currentModule).toBeNull();
    expect(harness.loader.activeSketchPath).toBeNull();
    expect(harness.revokeModuleUrl).toHaveBeenCalledWith("blob:broken");
  });

  it("rolls back URL creation failures without attempting revocation", async () => {
    const harness = createHarness([]);
    const urlError = new Error("URL creation failed");
    harness.createModuleUrl.mockImplementation(() => { throw urlError; });

    await harness.loader.load({ code: "broken" });

    expect(harness.scope.rollbackCandidateScope).toHaveBeenCalledOnce();
    expect(harness.dependencies.onLoadError).toHaveBeenCalledWith(urlError);
    expect(harness.revokeModuleUrl).not.toHaveBeenCalled();
  });

  it("serializes queued sketch updates", async () => {
    let resolveFirstImport: ((module: ImportedSketchModule<Scene, Config>) => void) | undefined;
    const firstImport = new Promise<ImportedSketchModule<Scene, Config>>((resolve) => {
      resolveFirstImport = resolve;
    });
    const harness = createHarness([]);
    harness.importModule
      .mockImplementationOnce(() => firstImport)
      .mockResolvedValueOnce({ setup: () => ({ mode: "second" }) });

    const firstLoad = harness.loader.enqueue({ code: "first" });
    const secondLoad = harness.loader.enqueue({ code: "second" });

    await vi.waitFor(() => expect(harness.importModule).toHaveBeenCalledTimes(1));
    expect(harness.importModule).toHaveBeenLastCalledWith("blob:first");

    resolveFirstImport?.({ setup: () => ({ mode: "first" }) });
    await Promise.all([firstLoad, secondLoad]);

    expect(harness.importModule).toHaveBeenCalledTimes(2);
    expect(harness.importModule).toHaveBeenNthCalledWith(2, "blob:second");
    expect(harness.loader.currentModule).not.toBeNull();
  });
});
