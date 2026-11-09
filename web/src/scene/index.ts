import * as THREE from 'three';
import { GRID_SIZE, layoutOf, selectionTile, tileKey, type Game, type GameState, type Tile } from '../store';
import { sceneBus, type SceneBus } from '../store/scene-bus';
import { createRig } from './camera';
import { rotateDelta, wheelFactor, ZOOM_MAX, ZOOM_MIN } from './cameraMath';
import { tileOf } from './coords';
import { EntityLayer } from './entityLayer';
import { entityAt, factsFor, planEntities, type EntityDesc } from './entities';
import { ctxFromSnapshot, emissionsFromSnapshot, planEmitters, planFlowLoop, planFrame, type Emitter, type FramePlan, type Track } from './flows';
import { buildModel, makeTintMaterial, setTint, type BuiltModel } from './geometry';
import { entityTints, legendFor, tileTints, type LensCtx } from './lenses';
import { FxLayer } from './fx';
import { MODELS } from './models';
import { pathTiles } from './paths';
import { ASPHALT, BACKGROUND, GROUND, HOVER_BAD, HOVER_OK, mix, SELECT, SOIL } from './palette';
import { TIMING } from './timing';

export { TIMING } from './timing';

export interface SceneHandle {
  dispose(): void;
  /** Test/debug hook: current instance counts and draw calls. */
  stats(): { drawCalls: number; triangles: number; instances: number; entities: number };
}

function marker(color: number, opacity: number) {
  const mat = new THREE.MeshBasicMaterial({ color, transparent: true, opacity, side: THREE.DoubleSide, depthTest: false });
  const m = new THREE.Mesh(new THREE.PlaneGeometry(0.98, 0.98), mat);
  m.rotation.x = -Math.PI / 2;
  m.renderOrder = 10;
  m.visible = false;
  return m;
}

function ringMarker(color: number) {
  const pts: number[] = [];
  const h = 0.49;
  pts.push(-h, 0, -h, h, 0, -h, h, 0, -h, h, 0, h, h, 0, h, -h, 0, h, -h, 0, h, -h, 0, -h);
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
  const l = new THREE.LineSegments(g, new THREE.LineBasicMaterial({ color, depthTest: false }));
  l.renderOrder = 11;
  l.visible = false;
  return l;
}

/** The 3D town. Reads and writes game state only through the store and the scene bus. */
export function createScene(container: HTMLElement, game: Game, bus: SceneBus = sceneBus): SceneHandle {
  const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)');
  const reducedMotion = () => !!reduced?.matches;

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(2, window.devicePixelRatio));
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap; // PCFSoftShadowMap is deprecated in three r18x
  const el = renderer.domElement;
  container.appendChild(el);
  el.style.display = 'block';
  el.style.touchAction = 'none';
  el.setAttribute('aria-label', 'Town map, 12 by 12 tiles. Tab to focus it. Arrow keys move the tile cursor along rows and columns, Enter selects or places, Escape cancels, Q and E rotate, plus and minus zoom.');
  el.setAttribute('role', 'application');
  el.tabIndex = 0;
  el.dataset.focus = 'map';

  const size = GRID_SIZE;
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(BACKGROUND);
  scene.add(new THREE.HemisphereLight(0xfff6e6, 0xc9bda5, 1.35));
  const sun = new THREE.DirectionalLight(0xfff0d8, 1.9);
  sun.position.set(-7, 15, 9);
  sun.castShadow = true;
  sun.shadow.mapSize.set(2048, 2048);
  Object.assign(sun.shadow.camera, { left: -10, right: 10, top: 10, bottom: -10, near: 1, far: 40 });
  sun.shadow.bias = -0.0004;
  sun.shadow.normalBias = 0.03;
  scene.add(sun);

  // Ground: a soil slab and 144 instanced tiles whose colours the lenses and build mode paint.
  const disposables: { dispose(): void }[] = [];
  const own = <T extends { dispose(): void }>(x: T): T => { disposables.push(x); return x; };
  const slab = new THREE.Mesh(own(new THREE.BoxGeometry(size + 0.5, 0.5, size + 0.5)), own(new THREE.MeshLambertMaterial({ color: SOIL })));
  slab.position.y = -0.31;
  slab.receiveShadow = true;
  scene.add(slab);
  const tilesMesh = new THREE.InstancedMesh(own(new THREE.BoxGeometry(0.97, 0.1, 0.97).translate(0, -0.05, 0)), own(new THREE.MeshLambertMaterial()), size * size);
  tilesMesh.receiveShadow = true;
  const dummy = new THREE.Object3D();
  for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
    dummy.position.set(x - (size - 1) / 2, 0.01, y - (size - 1) / 2);
    dummy.updateMatrix();
    tilesMesh.setMatrixAt(y * size + x, dummy.matrix);
    tilesMesh.setColorAt(y * size + x, new THREE.Color(GROUND[(x + y) % 2]));
  }
  scene.add(tilesMesh);

  const hoverFill = marker(HOVER_OK, 0.28);
  const hoverLine = ringMarker(HOVER_OK);
  const selectLine = ringMarker(SELECT);
  const selectFill = marker(SELECT, 0.16);
  scene.add(hoverFill, hoverLine, selectFill, selectLine);
  own({ dispose() { for (const m of [hoverFill, selectFill]) { m.geometry.dispose(); (m.material as THREE.Material).dispose(); } for (const l of [hoverLine, selectLine]) { l.geometry.dispose(); (l.material as THREE.Material).dispose(); } } });

  const layer = new EntityLayer();
  const fx = new FxLayer();
  scene.add(layer.group, fx.group);

  const ghostModels = new Map<string, BuiltModel>();
  const ghostMat = makeTintMaterial(0xffffff, 0.5);
  own(ghostMat);
  const ghost = new THREE.Mesh(new THREE.BufferGeometry(), ghostMat);
  ghost.visible = false;
  scene.add(ghost);

  const rig = createRig(size, reducedMotion);

  // ---- State -> scene ----------------------------------------------------------------------------------------
  let descs: EntityDesc[] = [];
  let roads = new Set<string>();
  let asphalt = new Set<string>();
  let last: GameState | null = null;
  let dirty = true;
  let flowLoop: Track[] = [];
  let ambient: Emitter[] = [];
  let plan: FramePlan | null = null;
  let planMonth: number | null = null;
  let monthStart = 0;
  const tileColors: THREE.Color = new THREE.Color();

  const lensCtx = (s: GameState): LensCtx => ({ descs, snapshot: s.snapshot, layout: layoutOf(s.snapshot), roads, asphalt });

  function paintTiles(s: GameState) {
    const base = tileTints(s.lens, lensCtx(s));
    const building = !!s.buildKind && s.phase === 'plan';
    for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
      let c = base[y * size + x]!;
      if (s.draft.some((d) => d.command.type === 'place' && d.command.kind === 'road' && d.command.tile.x === x && d.command.tile.y === y)) c = mix(c, ASPHALT, 0.5);
      if (building) c = mix(c, game.tileError({ x, y }) ? HOVER_BAD : HOVER_OK, game.tileError({ x, y }) ? 0.22 : 0.2);
      tilesMesh.setColorAt(y * size + x, tileColors.setHex(c));
    }
    tilesMesh.instanceColor!.needsUpdate = true;
  }

  function rebuildEntities(s: GameState) {
    const prev = new Map(descs.map((d) => [d.key, d]));
    const layout = layoutOf(s.snapshot);
    descs = planEntities(layout, s.snapshot, s.draft);
    roads = pathTiles(layout);
    asphalt = new Set([...layout.roads, ...layout.pendingRoads].map(tileKey));
    const pops = new Map<string, number>();
    for (const d of descs) {
      const p = prev.get(d.key);
      if (p && d.state === 'normal' && ((p.state === 'pending') || (p.model === 'firm_vacant' && d.model !== 'firm_vacant'))) pops.set(d.key, 0.5);
    }
    layer.rebuild(descs, pops);
    if (reducedMotion()) layer.finishPops();
    ambient = s.snapshot ? planEmitters(emissionsFromSnapshot(s.snapshot), ctxFromSnapshot(layoutOf(s.snapshot), s.snapshot)) : [];
    flowLoop = s.snapshot && s.lastResult && s.lastResult.months.length ? planFlowLoop(s.lastResult.months, ctxFromSnapshot(layoutOf(s.snapshot), s.snapshot)) : [];
  }

  function applyLens(s: GameState) {
    layer.setTints(entityTints(s.lens, lensCtx(s)));
    paintTiles(s);
    bus.legend.set(legendFor(s.lens, lensCtx(s)));
  }

  const keyAt = (t: Tile | null) => (t ? descs.find((d) => d.state !== 'draft' && d.ref.tile.x === t.x && d.ref.tile.y === t.y)?.key ?? null : null);

  function place(m: THREE.Object3D, t: Tile, y: number) { m.position.set(t.x - (size - 1) / 2, y, t.y - (size - 1) / 2); }

  function ghostFor(kind: keyof typeof MODELS) {
    let b = ghostModels.get(kind);
    if (!b) ghostModels.set(kind, (b = buildModel(MODELS[kind])));
    return b;
  }

  function syncPlayback(s: GameState) {
    const active = s.phase === 'playback' && !s.playback.skipped && s.playback.month >= 0 && !!s.lastResult && !!s.snapshot;
    if (!active) {
      if (plan) { plan = null; planMonth = null; }
      bus.playback.set({ active: s.phase === 'playback' && !s.playback.skipped, month: s.phase === 'playback' ? s.playback.month : -1, monthT: 0, overall: s.phase === 'playback' ? 0 : 1 });
      return;
    }
    if (planMonth !== s.playback.month) {
      planMonth = s.playback.month;
      monthStart = performance.now();
      const frame = s.lastResult!.months[planMonth];
      plan = frame ? planFrame(frame, ctxFromSnapshot(layoutOf(s.snapshot), s.snapshot!)) : null;
    }
  }

  const unsub = game.state.subscribe((s) => {
    const rebuilt = !last || last.draft !== s.draft || last.snapshot !== s.snapshot;
    if (rebuilt) rebuildEntities(s);
    if (rebuilt || last?.lens !== s.lens || last?.buildKind !== s.buildKind || last?.phase !== s.phase) applyLens(s);
    if (!last || last.camera !== s.camera) rig.setView(s.camera);
    if (last && last.phase === 'playback' && s.phase !== 'playback') layer.finishPops();
    syncPlayback(s);

    const selTile = selectionTile(s.snapshot, s.selection);
    selectLine.visible = selectFill.visible = !!selTile;
    if (selTile) { place(selectLine, selTile, 0.07); place(selectFill, selTile, 0.065); }
    layer.setSelected(keyAt(selTile));
    const ent = selTile ? entityAt(descs, selTile) : null;
    bus.selection.set(selTile ? { tile: selTile, entity: ent?.ref ?? null, facts: ent ? factsFor(ent) : [] } : null);

    const showHover = !!s.hover && s.phase !== 'playback';
    hoverLine.visible = hoverFill.visible = showHover;
    ghost.visible = false;
    layer.setHover(showHover ? keyAt(s.hover) : null);
    if (showHover && s.hover) {
      const reason = s.buildKind ? game.tileError(s.hover) : null;
      for (const m of [hoverFill, hoverLine]) { place(m, s.hover, 0.075); ((m as THREE.Mesh | THREE.LineSegments).material as THREE.MeshBasicMaterial).color.setHex(reason ? HOVER_BAD : HOVER_OK); }
      if (s.buildKind && s.buildKind !== 'road' && !reason) {
        const b = ghostFor(s.buildKind);
        ghost.geometry = b.geometry;
        setTint(ghostMat, 0xcfe8c8);
        place(ghost, s.hover, 0.06);
        ghost.visible = true;
      }
      bus.hover.set({ tile: s.hover, entity: entityAt(descs, s.hover)?.ref ?? null, invalidReason: reason, buildKind: s.buildKind });
    } else bus.hover.set(null);

    last = s;
    dirty = true;
  });

  // ---- Camera commands from the panels ------------------------------------------------------------------------
  const unCmd = bus.onCommand((c) => {
    if (c.type === 'zoom') rig.zoomBy(c.factor);
    else {
      rig.reset();
      const d = rotateDelta(game.state.get().camera, 0);
      for (let i = 0; i < Math.abs(d); i++) game.rotate(d < 0 ? -1 : 1);
    }
    dirty = true;
  });

  // ---- Pointer input ------------------------------------------------------------------------------------------
  const ray = new THREE.Raycaster();
  const ndcOf = (ev: { clientX: number; clientY: number }) => {
    const r = el.getBoundingClientRect();
    return { x: ((ev.clientX - r.left) / r.width) * 2 - 1, y: -((ev.clientY - r.top) / r.height) * 2 + 1 };
  };
  function pick(ev: { clientX: number; clientY: number }): Tile | null {
    const n = ndcOf(ev);
    ray.setFromCamera(new THREE.Vector2(n.x, n.y), rig.camera);
    const hit = ray.intersectObjects(layer.pickables, false)[0];
    if (hit) {
      const k = layer.keyOfMesh(hit.object);
      const d = descs.find((q) => q.key === k);
      if (d) return { ...d.ref.tile };
    }
    const g = rig.groundAt(n.x, n.y);
    return g ? tileOf(g.x, g.z, size) : null;
  }
  const pointers = new Map<number, { x: number; y: number }>();
  let down: { x: number; y: number; moved: boolean } | null = null;
  let pinch = 0;

  const onDown = (e: PointerEvent) => {
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    el.setPointerCapture?.(e.pointerId);
    down = { x: e.clientX, y: e.clientY, moved: false };
    if (pointers.size === 2) { const [a, b] = [...pointers.values()]; pinch = Math.hypot(a!.x - b!.x, a!.y - b!.y); down.moved = true; }
  };
  const onMove = (e: PointerEvent) => {
    const prev = pointers.get(e.pointerId);
    if (prev) pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
    if (pointers.size === 2 && pinch > 0) {
      const [a, b] = [...pointers.values()];
      const d = Math.hypot(a!.x - b!.x, a!.y - b!.y);
      const c = ndcOf({ clientX: (a!.x + b!.x) / 2, clientY: (a!.y + b!.y) / 2 });
      rig.zoomBy(d / pinch, c.x, c.y);
      pinch = d;
      dirty = true;
      return;
    }
    if (prev && down && pointers.size === 1) {
      if (!down.moved && Math.hypot(e.clientX - down.x, e.clientY - down.y) > 5) down.moved = true;
      if (down.moved) {
        const a = ndcOf({ clientX: prev.x, clientY: prev.y }), b = ndcOf(e);
        rig.pan(a.x, a.y, b.x, b.y);
        game.hover(null);
        dirty = true;
        return;
      }
    }
    if (!prev || !down) game.hover(pick(e));
  };
  const onUp = (e: PointerEvent) => {
    const wasClick = down && !down.moved && pointers.size === 1;
    pointers.delete(e.pointerId);
    if (pointers.size < 2) pinch = 0;
    if (wasClick) { const t = pick(e); if (t) game.clickTile(t); }
    if (pointers.size === 0) down = null;
  };
  const onLeave = () => { if (pointers.size === 0) game.hover(null); };
  const onWheel = (e: WheelEvent) => {
    e.preventDefault();
    const n = ndcOf(e);
    rig.zoomBy(wheelFactor(e.deltaY, e.deltaMode), n.x, n.y);
    dirty = true;
  };
  el.addEventListener('pointerdown', onDown);
  el.addEventListener('pointermove', onMove);
  el.addEventListener('pointerup', onUp);
  el.addEventListener('pointercancel', onUp);
  el.addEventListener('pointerleave', onLeave);
  el.addEventListener('wheel', onWheel, { passive: false });


  // ---- Keyboard: a tile cursor (the store's hover tile) for people who do not use a pointer ---------------------
  const ARROW_DIR: Record<string, number> = { ArrowRight: 0, ArrowDown: 1, ArrowLeft: 2, ArrowUp: 3 };
  const AXES: Tile[] = [{ x: 1, y: 0 }, { x: 0, y: 1 }, { x: -1, y: 0 }, { x: 0, y: -1 }];
  const describeTile = (t: Tile) => {
    const ent = entityAt(descs, t)?.ref;
    const build = game.state.get().buildKind;
    const err = build ? game.tileError(t) : null;
    return `Tile ${t.x}, ${t.y}: ${ent ? ent.label : 'empty'}${build ? (err ? `. Cannot build here: ${err}` : '. You can build here') : ''}`;
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const s = game.state.get();
    const arrow = ARROW_DIR[e.key];
    if (arrow !== undefined) {
      e.preventDefault();
      const d = AXES[(arrow - s.camera + 4) % 4]!;
      const cur = s.hover ?? selectionTile(s.snapshot, s.selection) ?? { x: Math.floor(size / 2), y: Math.floor(size / 2) };
      const t = { x: Math.min(size - 1, Math.max(0, cur.x + d.x)), y: Math.min(size - 1, Math.max(0, cur.y + d.y)) };
      game.hover(t);
      bus.announce.set(describeTile(t));
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      const t = s.hover ?? selectionTile(s.snapshot, s.selection);
      if (!t) { bus.announce.set('Move the cursor with the arrow keys first.'); return; }
      const err = game.clickTile(t);
      bus.announce.set(err ? `Cannot build at ${t.x}, ${t.y}: ${err}` : s.buildKind ? `Added to the plan at ${t.x}, ${t.y}` : describeTile(t));
    } else if (e.key === 'Escape') {
      if (s.buildKind) { game.setBuildKind(null); bus.announce.set('Build mode off'); e.stopPropagation(); }
      else if (s.selection) { game.select(null); bus.announce.set('Selection cleared'); e.stopPropagation(); }
    } else if (e.key === '+' || e.key === '=') { rig.zoomBy(1.25); dirty = true; }
    else if (e.key === '-' || e.key === '_') { rig.zoomBy(0.8); dirty = true; }
    else if (e.key === '0') { bus.requestReset(); }
  };
  const onFocus = () => { const s = game.state.get(); if (!s.hover) game.hover(selectionTile(s.snapshot, s.selection) ?? { x: Math.floor(size / 2), y: Math.floor(size / 2) }); };
  const onBlur = () => { if (pointers.size === 0) game.hover(null); };
  el.addEventListener('keydown', onKey);
  el.addEventListener('focus', onFocus);
  el.addEventListener('blur', onBlur);

  let contextLost = false;
  const onLost = (e: Event) => { e.preventDefault(); contextLost = true; bus.gl.set('lost'); };
  const onRestored = () => { contextLost = false; bus.gl.set('ok'); resize(); };
  el.addEventListener('webglcontextlost', onLost);
  el.addEventListener('webglcontextrestored', onRestored);

  const resize = () => {
    const w = container.clientWidth, h = container.clientHeight;
    if (w < 1 || h < 1) return;
    renderer.setSize(w, h);
    rig.resize(w, h);
    // Setting the size clears the canvas: draw straight away, or the next animation frame may be skipped and leave a blank view.
    if (!contextLost) renderer.render(scene, rig.camera);
    dirty = true;
  };
  const ro = new ResizeObserver(resize);
  ro.observe(container);
  resize();
  rig.snapNow();

  // ---- Frame loop ---------------------------------------------------------------------------------------------
  let raf = 0;
  let lastT = performance.now();
  let lastPaint = 0;
  let lastPublish = 0;
  let lastCam = '';
  const stats = () => ({ drawCalls: renderer.info.render.calls, triangles: renderer.info.render.triangles, instances: fx.instances, entities: layer.count });

  const loop = (now: number) => {
    raf = requestAnimationFrame(loop);
    const dt = Math.min(100, now - lastT);
    lastT = now;
    const sec = now / 1000;
    const s = game.state.get();
    const moving = rig.tick(dt);
    const popping = layer.update(dt / 1000, sec);
    const playing = !!plan;
    const monthT = playing ? (now - monthStart) / TIMING.monthMs : 0;
    const looping = s.lens === 'flows' && flowLoop.length > 0 && !playing;
    if (contextLost) return;
    const animating = moving || popping || playing || dirty;
    const ambientNeeded = ambient.length > 0 || layer.hasSpinners || looping;
    if (!animating && !(ambientNeeded && now - lastPaint > 40)) return;
    fx.update(plan, monthT, sec, ambient, looping ? flowLoop : [], (sec / 3) % 1, s.lens === 'emissions');
    renderer.render(scene, rig.camera);
    lastPaint = now;
    dirty = false;

    if (now - lastPublish > 120) {
      lastPublish = now;
      const cam = `${s.camera}|${rig.zoom.toFixed(2)}|${rig.panned}`;
      if (cam !== lastCam) {
        lastCam = cam;
        bus.camera.set({ angle: s.camera, zoom: rig.zoom, zoomMin: ZOOM_MIN, zoomMax: ZOOM_MAX, panned: rig.panned, reducedMotion: reducedMotion() });
      }
      if (s.phase === 'playback') {
        const m = s.playback.month;
        const mt = Math.min(1, Math.max(0, monthT));
        bus.playback.set({ active: !s.playback.skipped, month: m, monthT: mt, overall: m < 0 ? 0 : Math.min(1, (m + mt) / 3) });
      }
      bus.stats.set(stats());
    }
  };
  raf = requestAnimationFrame(loop);

  return {
    stats,
    dispose() {
      cancelAnimationFrame(raf);
      unsub();
      unCmd();
      ro.disconnect();
      el.removeEventListener('pointerdown', onDown);
      el.removeEventListener('pointermove', onMove);
      el.removeEventListener('pointerup', onUp);
      el.removeEventListener('pointercancel', onUp);
      el.removeEventListener('pointerleave', onLeave);
      el.removeEventListener('wheel', onWheel);
      el.removeEventListener('keydown', onKey);
      el.removeEventListener('focus', onFocus);
      el.removeEventListener('blur', onBlur);
      el.removeEventListener('webglcontextlost', onLost);
      el.removeEventListener('webglcontextrestored', onRestored);
      layer.dispose();
      fx.dispose();
      ghostModels.forEach((b) => b.geometry.dispose());
      disposables.forEach((d) => d.dispose());
      tilesMesh.dispose();
      renderer.dispose();
      el.remove();
    },
  };
}
