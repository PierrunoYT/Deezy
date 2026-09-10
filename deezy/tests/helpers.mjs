import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

export function loadModule(path, dependencies, globals = {}) {
  const source = readFileSync(new URL(path, import.meta.url), 'utf8');
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 }
  });
  const exports = {};
  vm.runInNewContext(outputText, {
    exports,
    require(name) {
      if (!(name in dependencies)) throw new Error(`Unexpected dependency: ${name}`);
      return dependencies[name];
    },
    console: { log() {}, warn() {}, error() {} },
    setTimeout, clearTimeout,
    ...globals
  }, { filename: path });
  return exports;
}

export function store(value) {
  return { value, set(next) { this.value = next; }, update(fn) { this.value = fn(this.value); } };
}

export function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

export const settle = () => new Promise(resolve => setImmediate(resolve));
