import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Subscribe to a Tauri event. Automatically cleans up on unmount.
 *
 * Uses a `cancelled` flag to handle the race between async `listen()` and
 * React StrictMode cleanup (which may run before the promise resolves).
 */
export function useOmniEvent<T>(
  eventName: string,
  handler: (payload: T) => void,
) {
  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;

    listen<T>(eventName, (event) => {
      if (!cancelled) {
        handler(event.payload);
      }
    }).then((fn) => {
      if (cancelled) {
        // Effect was cleaned up before listen() resolved -- unlisten immediately
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [eventName, handler]);
}
