interface StoppableTrack {
  stop(): void;
}

export interface AudioCaptureStream {
  getTracks(): StoppableTrack[];
}

export interface AudioCaptureContext {
  close(): Promise<void>;
}

export interface AudioCaptureResources<
  TStream extends AudioCaptureStream,
  TContext extends AudioCaptureContext,
> {
  stream: TStream;
  context: TContext;
}

interface StartAudioCaptureOptions<
  TStream extends AudioCaptureStream,
  TContext extends AudioCaptureContext,
> {
  acquireStream: () => Promise<TStream>;
  createContext: () => TContext;
  initialize: (stream: TStream, context: TContext) => void;
  onReleaseError?: (error: unknown) => void;
}

export async function releaseAudioCapture(
  resources: {
    stream: AudioCaptureStream | null;
    context: AudioCaptureContext | null;
  },
  onError: (error: unknown) => void = () => undefined,
): Promise<void> {
  if (resources.stream) {
    for (const track of resources.stream.getTracks()) {
      try {
        track.stop();
      } catch (error: unknown) {
        onError(error);
      }
    }
  }

  if (resources.context) {
    try {
      await resources.context.close();
    } catch (error: unknown) {
      onError(error);
    }
  }
}

export async function startAudioCapture<
  TStream extends AudioCaptureStream,
  TContext extends AudioCaptureContext,
>(
  options: StartAudioCaptureOptions<TStream, TContext>,
): Promise<AudioCaptureResources<TStream, TContext>> {
  const stream = await options.acquireStream();
  let context: TContext | null = null;

  try {
    context = options.createContext();
    options.initialize(stream, context);
    return { stream, context };
  } catch (error: unknown) {
    await releaseAudioCapture(
      { stream, context },
      options.onReleaseError,
    );
    throw error;
  }
}
