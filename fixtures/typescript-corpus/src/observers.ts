type Listener = () => number;

const callbacks = new Set<Listener>();
export function subscribe(callback: Listener) { callbacks.add(callback); }
export function notify() { for (const callback of callbacks) callback(); }
export function redraw() { return 1; }
export function logUpdate() { return 2; }
subscribe(redraw);
subscribe(logUpdate);

class Scene {
  private readonly callbacks = new Set<Listener>();
  onUpdate(callback: Listener) { this.callbacks.add(callback); }
  triggerUpdate() { for (const callback of this.callbacks) callback(); }
}
export const scene = new Scene();
scene.onUpdate(redraw);

export class StaticBus {
  private static readonly callbacks = new Set<Listener>();
  static onUpdate(callback: Listener) { StaticBus.callbacks.add(callback); }
  static triggerUpdate() { for (const callback of StaticBus.callbacks) callback(); }
}
StaticBus.onUpdate(logUpdate);

class MultipleInstances {
  private readonly callbacks = new Set<Listener>();
  onUpdate(callback: Listener) { this.callbacks.add(callback); }
  triggerUpdate() { for (const callback of this.callbacks) callback(); }
}
const first = new MultipleInstances();
const second = new MultipleInstances();
first.onUpdate(redraw);
second.triggerUpdate();
