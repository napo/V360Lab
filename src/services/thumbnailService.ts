import { cameraService } from "./cameraService";

/**
 * Thumbnails are fetched by the backend and returned as data URLs.
 * Results are cached per URL and requests are limited so that opening the
 * media page does not flood the camera's small HTTP server.
 */
const MAX_CONCURRENT = 3;
const cache = new Map<string, Promise<string>>();
const queue: Array<() => void> = [];
let active = 0;

function schedule<T>(task: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const run = () => {
      active += 1;
      task()
        .then(resolve, reject)
        .finally(() => {
          active -= 1;
          queue.shift()?.();
        });
    };
    if (active < MAX_CONCURRENT) run();
    else queue.push(run);
  });
}

export function loadThumbnail(url: string): Promise<string> {
  let pending = cache.get(url);
  if (!pending) {
    pending = schedule(() => cameraService.thumbnail(url));
    // Do not cache failures: allow a retry on the next render.
    pending.catch(() => cache.delete(url));
    cache.set(url, pending);
  }
  return pending;
}

export function clearThumbnailCache(): void {
  cache.clear();
}
