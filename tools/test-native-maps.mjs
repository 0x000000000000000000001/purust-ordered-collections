// Differential runner for the native `Data.Map.Internal` primitives in a
// generated Purust workspace.
//
// The generated crate contains BOTH lanes, so no candidate source is injected:
//   - `Data_Map_Internal_{insert,insertWith,unionWith}PS`: the explicit
//     PureScript oracles (comparator as an argument, no `Ord` dictionary);
//   - `Data_Map_Internal_{insert,insertWith,unionWith}Impl`: the native exports
//     appended from `src/Data/Map/Internal.rs`.
// The fixture compares them directly. A workspace whose source embeds the
// native lane is expected and fine.
//
// When `purust_core` provides `Value::ClassShared`, the fixture also exercises
// the shared-owner key representation through `--cfg purust_class_shared`.
//
// Usage:
//   node tools/test-native-maps.mjs GENERATED_RUST
//   PURUST_GENERATED_RUST=/path/to/rust node tools/test-native-maps.mjs
//
// On failure the fixture workspace (sources + build.log/run.log) is kept and
// its path printed; on success it is removed unless PURUST_NATIVE_KEEP=1.
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const rust = resolve(process.argv[2] ?? process.env.PURUST_GENERATED_RUST ?? '');
const target = resolve(process.env.PURUST_NATIVE_TARGET ?? join(rust, 'target'));
assert(process.argv[2] || process.env.PURUST_GENERATED_RUST,
  'Usage: node tools/test-native-maps.mjs GENERATED_RUST (or PURUST_GENERATED_RUST)');

const oracleSource = readFileSync(join(rust, 'Purs_Data_Map_Internal/src/lib.rs'), 'utf8');
for (const symbol of [
  'Data_Map_Internal_insertPS(', 'Data_Map_Internal_insertWithPS(', 'Data_Map_Internal_unionWithPS(',
  'Data_Map_Internal_insertImpl(', 'Data_Map_Internal_insertWithImpl(', 'Data_Map_Internal_unionWithImpl(',
]) {
  assert.ok(oracleSource.includes(symbol), `Missing ${symbol} in the generated crate`);
}
const coreSource = readFileSync(join(rust, 'purust_core/src/lib.rs'), 'utf8');
const classShared = coreSource.includes('ClassShared(');

const directory = mkdtempSync(join(tmpdir(), 'purust-native-maps-'));
let failed = false;
try {
  mkdirSync(join(directory, 'src'));
  const modules = ['purust_core', 'Purs_Data_Map_Internal', 'Purs_Data_Maybe', 'Purs_Data_Ord',
    'Purs_Data_Ordering', 'Purs_PureScript_Backend_Optimizer_CoreFn'];
  writeFileSync(join(directory, 'Cargo.toml'), '[package]\nname = "purust_native_maps_test"\nversion = "0.0.0"\nedition = "2021"\n' +
    '[profile.release]\nopt-level = 3\ndebug = false\nlto = false\n[dependencies]\n' +
    modules.map(name => `${name} = { path = ${JSON.stringify(join(rust, name))} }\n`).join(''));
  writeFileSync(join(directory, 'src/main.rs'), readFileSync(new URL('./test-native-maps.rs', import.meta.url), 'utf8'));
  console.log(`ClassShared fixture: ${classShared ? 'enabled' : 'unavailable in purust_core'}`);
  const build = spawnSync('cargo', ['rustc', '--offline', '--release', '--quiet',
    '--manifest-path', join(directory, 'Cargo.toml'), '--target-dir', target,
    '--bin', 'purust_native_maps_test', ...(classShared ? ['--', '--cfg', 'purust_class_shared'] : [])],
    { encoding: 'utf8', timeout: 300000, maxBuffer: 8 * 1024 * 1024 });
  if (build.status !== 0) {
    writeFileSync(join(directory, 'build.log'), `${build.stdout ?? ''}${build.stderr ?? ''}`);
    throw new Error(build.error?.message ?? `cargo build failed (${build.status})`);
  }
  const run = spawnSync(join(target, 'release/purust_native_maps_test'), [], { encoding: 'utf8', timeout: 120000 });
  if (run.status !== 0) {
    writeFileSync(join(directory, 'run.log'), `${run.stdout ?? ''}${run.stderr ?? ''}`);
    throw new Error(run.error?.message ?? `native fixture failed (${run.status})`);
  }
  console.log(run.stdout.trim());
} catch (error) {
  failed = true;
  console.error(`native fixture workspace kept: ${directory}`);
  throw error;
} finally {
  if (!failed && process.env.PURUST_NATIVE_KEEP !== '1' && process.env.PBO_NATIVE_KEEP !== '1') {
    rmSync(directory, { recursive: true, force: true });
  }
}
