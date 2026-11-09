import * as THREE from 'three';

const OFFSET = new THREE.Vector3(14, 12, 14);

/** Orthographic camera orbiting the map centre in 90 degree snaps, eased. */
export function createRig() {
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 100);
  let angle = 0; // radians, eased
  let target = 0;
  const half = 8;

  function place() {
    const p = OFFSET.clone().applyAxisAngle(new THREE.Vector3(0, 1, 0), angle);
    camera.position.copy(p);
    camera.lookAt(0, 0, 0);
  }
  return {
    camera,
    setSnap(n: number) {
      const d = n * (Math.PI / 2) - target;
      target += Math.atan2(Math.sin(d), Math.cos(d)); // shortest way round
    },
    resize(w: number, h: number) {
      const aspect = w / Math.max(1, h);
      camera.left = -half * aspect; camera.right = half * aspect; camera.top = half; camera.bottom = -half;
      camera.updateProjectionMatrix();
    },
    /** Returns true while still moving. */
    tick(): boolean {
      const d = target - angle;
      angle = Math.abs(d) < 0.002 ? target : angle + d * 0.2;
      place();
      return Math.abs(target - angle) > 0.001;
    },
    snapNow() { angle = target; place(); },
  };
}
