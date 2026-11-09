import * as THREE from 'three';
import type { Game, GameState, Tile } from '../store';
import { createRig } from './camera';
import { tileOf, worldOf } from './coords';
import { buildEntities } from './entities';
import { GROUND, HOVER_BAD, HOVER_OK, SELECT, INFRA } from './palette';

export interface SceneHandle { dispose(): void }

function marker(color: number, filled: boolean) {
  const mat = new THREE.MeshBasicMaterial({ color, transparent: true, opacity: filled ? 0.45 : 0.9, side: THREE.DoubleSide, depthTest: false });
  const m = new THREE.Mesh(new THREE.PlaneGeometry(0.96, 0.96), mat);
  m.rotation.x = -Math.PI / 2;
  m.renderOrder = 10;
  m.visible = false;
  return m;
}

/** Placeholder 3D scene. Reads and writes game state only through the store. */
export function createScene(container: HTMLElement, game: Game): SceneHandle {
  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(2, window.devicePixelRatio));
  container.appendChild(renderer.domElement);
  renderer.domElement.style.display = 'block';
  renderer.domElement.setAttribute('aria-label', 'Town map, 12 by 12 tiles. Use Q and E to rotate.');

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0xf1f3ec);
  scene.add(new THREE.AmbientLight(0xffffff, 1.6));
  const sun = new THREE.DirectionalLight(0xffffff, 1.4);
  sun.position.set(6, 12, 4);
  scene.add(sun);

  const size = game.state.get().layout.gridSize;
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const m = new THREE.Mesh(new THREE.PlaneGeometry(0.98, 0.98), new THREE.MeshLambertMaterial({ color: GROUND[(x + y) % 2] }));
      m.rotation.x = -Math.PI / 2;
      const [wx, , wz] = worldOf({ x, y }, size);
      m.position.set(wx, 0, wz);
      scene.add(m);
    }
  }
  const hoverMark = marker(HOVER_OK, true);
  const selectMark = marker(SELECT, false);
  scene.add(hoverMark, selectMark);
  const ghost = new THREE.Mesh(new THREE.BoxGeometry(0.8, 1, 0.8), new THREE.MeshLambertMaterial({ color: 0xffffff, transparent: true, opacity: 0.5 }));
  ghost.visible = false;
  scene.add(ghost);

  let entities = new THREE.Group();
  scene.add(entities);
  const rig = createRig();
  let last: GameState | null = null;
  let dirty = true;

  const place = (m: THREE.Object3D, t: Tile, y = 0.03) => {
    const [x, , z] = worldOf(t, size, y);
    m.position.set(x, y, z);
  };

  const unsub = game.state.subscribe((s) => {
    if (!last || last.layout !== s.layout || last.draft !== s.draft || last.snapshot !== s.snapshot) {
      scene.remove(entities);
      entities = buildEntities(s, s.snapshot?.turn ?? 1);
      scene.add(entities);
    }
    if (!last || last.camera !== s.camera) rig.setSnap(s.camera);
    selectMark.visible = !!s.selection;
    if (s.selection) place(selectMark, s.selection);
    const showHover = !!s.hover && s.phase !== 'playback';
    hoverMark.visible = showHover;
    ghost.visible = false;
    if (showHover && s.hover) {
      place(hoverMark, s.hover, 0.04);
      const bad = s.buildKind ? !!game.tileError(s.hover) : false;
      (hoverMark.material as THREE.MeshBasicMaterial).color.setHex(bad ? HOVER_BAD : HOVER_OK);
      if (s.buildKind && !bad) {
        const h = INFRA[s.buildKind].height;
        ghost.scale.y = h;
        (ghost.material as THREE.MeshLambertMaterial).color.setHex(INFRA[s.buildKind].color);
        place(ghost, s.hover, h / 2 + 0.02);
        ghost.visible = true;
      }
    }
    last = s;
    dirty = true;
  });

  const ray = new THREE.Raycaster();
  const ground = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
  function pick(ev: PointerEvent): Tile | null {
    const r = renderer.domElement.getBoundingClientRect();
    const ndc = new THREE.Vector2(((ev.clientX - r.left) / r.width) * 2 - 1, -((ev.clientY - r.top) / r.height) * 2 + 1);
    ray.setFromCamera(ndc, rig.camera);
    const hit = new THREE.Vector3();
    return ray.ray.intersectPlane(ground, hit) ? tileOf(hit.x, hit.z, size) : null;
  }
  const onMove = (e: PointerEvent) => game.hover(pick(e));
  const onLeave = () => game.hover(null);
  const onClick = (e: PointerEvent) => {
    const t = pick(e);
    if (t) game.clickTile(t);
  };
  const el = renderer.domElement;
  el.addEventListener('pointermove', onMove);
  el.addEventListener('pointerleave', onLeave);
  el.addEventListener('click', onClick as unknown as EventListener);

  const resize = () => {
    const w = container.clientWidth, h = container.clientHeight;
    renderer.setSize(w, h);
    rig.resize(w, h);
    dirty = true;
  };
  const ro = new ResizeObserver(resize);
  ro.observe(container);
  resize();
  rig.snapNow();

  let raf = 0;
  const loop = () => {
    raf = requestAnimationFrame(loop);
    if (rig.tick() || dirty) {
      renderer.render(scene, rig.camera);
      dirty = false;
    }
  };
  loop();

  return {
    dispose() {
      cancelAnimationFrame(raf);
      unsub();
      ro.disconnect();
      el.removeEventListener('pointermove', onMove);
      el.removeEventListener('pointerleave', onLeave);
      el.removeEventListener('click', onClick as unknown as EventListener);
      renderer.dispose();
      el.remove();
    },
  };
}
