import { execSync } from 'child_process';
import fs from 'fs';

console.log('Building Rust WebAssembly module (release)...');
execSync('cargo build --manifest-path wasm/Cargo.toml --target wasm32-unknown-unknown --release', { stdio: 'inherit' });

const src = 'wasm/target/wasm32-unknown-unknown/release/wsm.wasm';
const dest = 'public/wsm.wasm';

if (fs.existsSync(src)) {
  fs.copyFileSync(src, dest);
  const stats = fs.statSync(dest);
  console.log(`Successfully built and copied wsm.wasm (${(stats.size / 1024).toFixed(1)} KB) to public/`);
} else {
  console.error(`Error: Compiled binary not found at ${src}`);
  process.exit(1);
}
