export interface SketchLoadPayload<TFxSettings> {
  code: string;
  dir?: string;
  sketchPath?: string;
  fxSettings?: TFxSettings;
}

export interface ImportedSketchModule<TScene, TConfig> {
  setup?: (this: Record<string, unknown>, scene: TScene) => TConfig | void;
  update?: (this: Record<string, unknown>, context: unknown) => void;
  cleanup?: (this: Record<string, unknown>, scene: TScene) => void;
}

export interface ActiveSketchModule<TScene> {
  update: (context: unknown) => void;
  cleanup: (scene: TScene) => void;
}

interface CandidateScopeBoundary {
  disposeActiveScope: () => void;
  beginCandidateScope: () => void;
  commitCandidateScope: () => void;
  rollbackCandidateScope: () => void;
}

export interface SketchLoaderDependencies<TScene, TConfig, TFxSettings> {
  ready: PromiseLike<void>;
  scene: TScene;
  scope: CandidateScopeBoundary;
  createModuleUrl: (code: string) => string;
  revokeModuleUrl: (url: string) => void;
  importModule: (url: string) => Promise<ImportedSketchModule<TScene, TConfig>>;
  setSketchDirectory: (directory: string) => void;
  onSetupConfig: (config: TConfig | undefined) => void;
  onModuleConfigured: (config: TConfig | undefined) => void;
  applyFxSettings: (settings: TFxSettings) => void;
  onCleanupError: (error: unknown) => void;
  onLoadError: (error: unknown) => void;
  onUnexpectedError: (error: unknown) => void;
}

export class SketchLoader<TScene, TConfig, TFxSettings> {
  private module: ActiveSketchModule<TScene> | null = null;
  private sketchPath: string | null = null;
  private queue: Promise<void> = Promise.resolve();

  constructor(private readonly dependencies: SketchLoaderDependencies<TScene, TConfig, TFxSettings>) {}

  get currentModule(): ActiveSketchModule<TScene> | null {
    return this.module;
  }

  get activeSketchPath(): string | null {
    return this.sketchPath;
  }

  enqueue(payload: SketchLoadPayload<TFxSettings>): Promise<void> {
    this.queue = this.queue
      .then(() => this.load(payload))
      .catch((error: unknown) => {
        this.dependencies.onUnexpectedError(error);
      });
    return this.queue;
  }

  async load(payload: SketchLoadPayload<TFxSettings>): Promise<void> {
    const {
      ready,
      scene,
      scope,
      createModuleUrl,
      revokeModuleUrl,
      importModule,
      setSketchDirectory,
      onSetupConfig,
      onModuleConfigured,
      applyFxSettings,
      onCleanupError,
      onLoadError,
    } = this.dependencies;

    await ready;

    const previousModule = this.module;
    this.module = null;
    this.sketchPath = null;
    if (previousModule) {
      try {
        previousModule.cleanup(scene);
      } catch (error: unknown) {
        onCleanupError(error);
      } finally {
        scope.disposeActiveScope();
      }
    } else {
      scope.disposeActiveScope();
    }

    scope.beginCandidateScope();
    let moduleUrl: string | null = null;
    try {
      if (payload.dir) setSketchDirectory(payload.dir);

      moduleUrl = createModuleUrl(payload.code);
      const userModule = await importModule(moduleUrl);
      const sketchContext: Record<string, unknown> = {};
      let sketchConfig: TConfig | undefined;

      if (typeof userModule.setup === "function") {
        const config = userModule.setup.call(sketchContext, scene);
        sketchConfig = config ?? undefined;
        onSetupConfig(sketchConfig);
      }

      onModuleConfigured(sketchConfig);
      scope.commitCandidateScope();
      this.module = {
        update: (context: unknown) => userModule.update?.call(sketchContext, context),
        cleanup: (targetScene: TScene) => userModule.cleanup?.call(sketchContext, targetScene),
      };
      this.sketchPath = payload.sketchPath ?? null;

      if (payload.fxSettings !== undefined) applyFxSettings(payload.fxSettings);
    } catch (error: unknown) {
      scope.rollbackCandidateScope();
      onModuleConfigured(undefined);
      onLoadError(error);
    } finally {
      if (moduleUrl) revokeModuleUrl(moduleUrl);
    }
  }
}
