import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import type { Model, Part, Shape } from './models';

const unit = new Map<Shape, THREE.BufferGeometry>();

function makeUnit(s: Shape): THREE.BufferGeometry {
  let g: THREE.BufferGeometry;
  switch (s) {
    case 'box': g = new THREE.BoxGeometry(1, 1, 1); break;
    case 'cyl': g = new THREE.CylinderGeometry(0.5, 0.5, 1, 10); break;
    case 'cone': g = new THREE.ConeGeometry(0.5, 1, 9); break;
    case 'pyr': g = new THREE.ConeGeometry(0.7, 1, 4); g.rotateY(Math.PI / 4); break;
    case 'sphere': g = new THREE.SphereGeometry(0.5, 10, 8); break;
    case 'octa': g = new THREE.OctahedronGeometry(0.5); break;
    case 'gable': {
      const sh = new THREE.Shape();
      sh.moveTo(-0.5, 0); sh.lineTo(0.5, 0); sh.lineTo(0, 1); sh.closePath();
      g = new THREE.ExtrudeGeometry(sh, { depth: 1, bevelEnabled: false });
      g.translate(0, 0, -0.5);
      break;
    }
  }
  if (s !== 'gable') g.translate(0, 0.5, 0);
  const out = g.index ? g.toNonIndexed() : g;
  out.deleteAttribute('uv');
  if (g !== out) g.dispose();
  return out;
}

function partGeometry(p: Part): THREE.BufferGeometry {
  let u = unit.get(p.s);
  if (!u) unit.set(p.s, (u = makeUnit(p.s)));
  const g = u.clone();
  const m = new THREE.Matrix4().compose(
    new THREE.Vector3(...p.p),
    new THREE.Quaternion().setFromEuler(new THREE.Euler(0, p.ry ?? 0, p.rz ?? 0, 'YXZ')),
    new THREE.Vector3(...p.z),
  );
  g.applyMatrix4(m);
  const n = g.attributes.position!.count;
  const col = new THREE.Color(p.tint ? 0xffffff : p.c);
  const colors = new Float32Array(n * 3);
  for (let i = 0; i < n; i++) { colors[i * 3] = col.r; colors[i * 3 + 1] = col.g; colors[i * 3 + 2] = col.b; }
  g.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  g.setAttribute('tintW', new THREE.BufferAttribute(new Float32Array(n).fill(p.tint ? 1 : 0), 1));
  return g;
}

/** Merge a model's parts into one geometry (one draw call per building). Vertex colour + `tintW` mask for the lens tint. */
export function mergeParts(parts: Part[]): THREE.BufferGeometry {
  const gs = parts.map(partGeometry);
  const merged = mergeGeometries(gs, false);
  gs.forEach((g) => g.dispose());
  if (!merged) throw new Error('empty model');
  return merged;
}

export interface BuiltModel { geometry: THREE.BufferGeometry; edges: Float32Array; spin?: { geometry: THREE.BufferGeometry; at: [number, number, number] } }

/** Geometry for a model plus its outline segments (object space). */
export function buildModel(m: Model): BuiltModel {
  const geometry = mergeParts(m.parts);
  const e = new THREE.EdgesGeometry(geometry, 38);
  const edges = Float32Array.from(e.attributes.position!.array as ArrayLike<number>);
  e.dispose();
  return { geometry, edges, spin: m.spin ? { geometry: mergeParts(m.spin.parts), at: m.spin.at } : undefined };
}


/**
 * Lambert material whose "tint" parts take a per-material colour, so a lens recolours walls without touching roofs and details.
 * All instances share one compiled program (customProgramCacheKey).
 */
export function makeTintMaterial(tint: number, opacity = 1): THREE.MeshLambertMaterial {
  const mat = new THREE.MeshLambertMaterial({ vertexColors: true, transparent: opacity < 1, opacity, depthWrite: opacity >= 1 });
  const uTint = { value: new THREE.Color(tint) };
  mat.userData.uTint = uTint;
  mat.onBeforeCompile = (sh) => {
    sh.uniforms.uTint = uTint;
    sh.vertexShader = sh.vertexShader
      .replace('#include <common>', '#include <common>\nattribute float tintW;\nvarying float vTintW;')
      .replace('#include <begin_vertex>', '#include <begin_vertex>\nvTintW = tintW;');
    sh.fragmentShader = sh.fragmentShader
      .replace('#include <common>', '#include <common>\nuniform vec3 uTint;\nvarying float vTintW;')
      .replace('#include <color_fragment>', '#include <color_fragment>\ndiffuseColor.rgb = mix(diffuseColor.rgb, uTint, vTintW);');
  };
  mat.customProgramCacheKey = () => 'tint-lambert';
  return mat;
}
export const setTint = (mat: THREE.Material, hex: number) => (mat.userData.uTint as { value: THREE.Color }).value.setHex(hex);
