import { useState, useEffect } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { readTextFile, watch } from "@tauri-apps/plugin-fs";
import { emit } from "@tauri-apps/api/event";
import { FileCode, AlertCircle, FileAudio, Settings } from "lucide-react";

export default function App() {
  const [filePath, setFilePath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!filePath) return;

    let unwatch: (() => void) | null = null;
    let lastEmitTime = 0;
    const THROTTLE_MS = 150;

    const loadAndEmit = async () => {
      try {
        const code = await readTextFile(filePath);
        await emit("user-code-update", { code });
        console.log("Emitted user code update.");
        setError(null);
      } catch (err: any) {
        console.error("Failed to read or emit file:", err);
        setError(`Failed to read file: ${err.message || err}`);
      }
    };

    // Initial load
    loadAndEmit();

    // Start watching
    watch(
      filePath,
      (event) => {
        if (
          event.type === "any" ||
          event.type === "other" ||
          (typeof event.type === "object" && "modify" in event.type)
        ) {
          const now = Date.now();
          if (now - lastEmitTime > THROTTLE_MS) {
            lastEmitTime = now;
            loadAndEmit();
          }
        }
      },
      { recursive: false, delayMs: 20 }
    ).then((unwatchFn) => {
      unwatch = unwatchFn;
    }).catch((err: any) => {
      console.error("Failed to start watcher:", err);
      setError(`Failed to start watcher: ${err.message || err}`);
    });

    return () => {
      if (unwatch) unwatch();
    };
  }, [filePath]);

  const handleOpenFile = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: "JavaScript", extensions: ["js"] }],
      });

      if (selected && typeof selected === "string") {
        setFilePath(selected);
      }
    } catch (err: any) {
      console.error("Dialog error:", err);
      setError(`Dialog failed: ${err.message || err}`);
    }
  };

  return (
    <div className="min-h-screen bg-neutral-100 dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100 flex flex-col items-center justify-center p-6 font-sans transition-colors duration-200">
      <div className="max-w-md w-full bg-white dark:bg-neutral-800 rounded-2xl shadow-xl overflow-hidden border border-neutral-200 dark:border-neutral-700">
        
        <div className="p-6 border-b border-neutral-200 dark:border-neutral-700 flex items-center justify-center gap-3">
          <Settings className="w-6 h-6 text-blue-500" />
          <h1 className="text-2xl font-bold tracking-tight">shekere Control Panel</h1>
        </div>

        <div className="p-8 flex flex-col items-center gap-6">
          <button
            onClick={handleOpenFile}
            className="w-full flex justify-center items-center gap-2 bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white px-6 py-3.5 rounded-xl font-medium transition-all shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-neutral-800"
          >
            <FileCode className="w-5 h-5" />
            Select JS File
          </button>

          <div className="w-full flex flex-col gap-2">
            <h2 className="text-sm font-semibold text-neutral-500 dark:text-neutral-400 uppercase tracking-wider">
              Currently Watching
            </h2>
            <div className="bg-neutral-100 dark:bg-neutral-900/50 p-4 rounded-xl text-sm font-mono break-all text-neutral-700 dark:text-neutral-300 border border-neutral-200 dark:border-neutral-700/50 flex items-start gap-3">
              <FileAudio className="w-5 h-5 shrink-0 text-neutral-400 mt-0.5" />
              <span>{filePath || "None"}</span>
            </div>
          </div>

          {error && (
            <div className="w-full flex items-start gap-3 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 p-4 rounded-xl text-sm border border-red-200 dark:border-red-900/50">
              <AlertCircle className="w-5 h-5 shrink-0" />
              <p>{error}</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
