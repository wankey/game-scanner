import { writable } from "svelte/store";

export interface Toast {
  id: number;
  kind: "info" | "ok" | "error";
  message: string;
}

let nextId = 1;

export const toasts = writable<Toast[]>([]);

export function pushToast(message: string, kind: Toast["kind"] = "info", ttl = 4000) {
  const id = nextId++;
  toasts.update((list) => [...list, { id, kind, message }]);
  setTimeout(() => {
    toasts.update((list) => list.filter((t) => t.id !== id));
  }, ttl);
}

export const toast = {
  info: (m: string) => pushToast(m, "info"),
  ok: (m: string) => pushToast(m, "ok"),
  error: (m: string) => pushToast(m, "error", 6000),
};
