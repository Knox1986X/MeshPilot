"""VRM рендер для MeshPilot (3D Втубер-модели).

VRM — 3D формат (VRoid Studio, Pixiv), стандарт для 3D Втуберов.
Работает через QWebEngineView + three.js + @pixiv/three-vrm (CDN).

Модель: один файл .vrm (меш + текстуры + blendshapes + анимации внутри).
Управление: blendshapes (blink, smile, mouth), взгляд, idle-анимация.
"""
from __future__ import annotations
import logging
from pathlib import Path

log = logging.getLogger("meshpilot.vrm")

VRM_HTML = """<!DOCTYPE html>
<html><head><meta charset="utf-8">
<style>html,body{margin:0;padding:0;background:transparent;overflow:hidden}
canvas{display:block}</style>
<script type="importmap">
{"imports": {"three": "https://unpkg.com/three@0.160.0/build/three.module.js",
"three/addons/": "https://unpkg.com/three@0.160.0/examples/jsm/",
"@pixiv/three-vrm": "https://unpkg.com/@pixiv/three-vrm@3.0.0/lib/three-vrm.module.js"}}
</script>
</head><body>
<script type="module">
import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { VRMUtils } from '@pixiv/three-vrm';

let vrm = null;
let scene, camera, renderer;
let mouthOpen = 0;

async function loadVRM(url) {
  scene = new THREE.Scene();
  camera = new THREE.PerspectiveCamera(30, 1, 0.1, 100);
  camera.position.set(0, 1.4, 2.5);
  renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
  renderer.setSize(400, 500);
  document.body.appendChild(renderer.domElement);
  const light = new THREE.DirectionalLight(0xffffff, 2);
  light.position.set(1, 2, 2);
  scene.add(light);
  scene.add(new THREE.AmbientLight(0xffffff, 0.8));
  const loader = new GLTFLoader();
  const gltf = await loader.loadAsync(url);
  const { VRM } = await import('@pixiv/three-vrm');
  vrm = await VRM.from(gltf);
  scene.add(vrm.scene);
  VRMUtils.rotateVRM0(vrm);
  animate();
  return "ok";
}

function animate() {
  requestAnimationFrame(animate);
  if (vrm) {
    // idle: лёгкое покачивание
    const t = Date.now() * 0.001;
    vrm.scene.rotation.y = Math.sin(t * 0.5) * 0.1;
    vrm.scene.position.y = Math.sin(t * 2) * 0.02;
    // blink каждые ~3 сек
    const blink = (t % 3.2) < 0.12 ? 1.0 : 0.0;
    try {
      vrm.expressionManager.setValue('blink', blink);
      vrm.expressionManager.setValue('aa', mouthOpen);
    } catch(e) {}
    vrm.update(0.016);
  }
  renderer.render(scene, camera);
}

function setMouth(v) { mouthOpen = Math.max(0, Math.min(1, v)); }
function setExpression(name) {
  try { if (vrm) vrm.expressionManager.setValue(name, 1.0); } catch(e) {}
}
</script>
</body></html>
"""

class VRMModel:
    def __init__(self, model_path: str | Path) -> None:
        self.path = Path(model_path)
        if not self.path.exists():
            raise FileNotFoundError(f"VRM модель не найдена: {self.path}")
        if self.path.suffix.lower() != ".vrm":
            raise ValueError(f"VRM модель должна быть .vrm, получено: {self.path}")
        log.info("VRM модель: %s (%.1f MB)", self.path.name, self.path.stat().st_size / 1e6)

    @property
    def is_valid(self) -> bool:
        return self.path.exists() and self.path.stat().st_size > 1000

    def model_url(self) -> str:
        return self.path.as_uri()
