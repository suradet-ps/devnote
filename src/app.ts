import { dev } from '$app/environment';

export const prerender = true;
export const ssr = false;

if (dev) {
  // @ts-expect-error Tauri global
  window.__TAURI__ && console.log('Tauri dev mode');
}
