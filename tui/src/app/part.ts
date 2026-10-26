import type { App } from "../app.ts";

/** Types the `this` of the methods of one part of the App, which app.ts joins into the class. */
export function part<T>(methods: T & ThisType<App>): T {
  return methods;
}

/** Puts the methods of each part on the prototype of the class, as methods of the class are. */
export function assemble(target: { prototype: object }, ...parts: object[]): void {
  for (const one of parts) {
    for (const [name, descriptor] of Object.entries(Object.getOwnPropertyDescriptors(one))) {
      if (Object.hasOwn(target.prototype, name)) throw new Error(`${name} is in two parts of the App`);
      Object.defineProperty(target.prototype, name, { ...descriptor, enumerable: false });
    }
  }
}

/** Binds the named methods to the instance, as the arrow functions of the class were. */
export function bindAll(app: object, names: readonly string[]): true {
  const own = app as Record<string, unknown>;
  for (const name of names) {
    const method = own[name];
    if (typeof method !== "function") throw new Error(`${name} is no method of the App`);
    own[name] = method.bind(app);
  }
  return true;
}
