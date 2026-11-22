// Live2D загрузчик (PixiJS + Cubism 4) для Tauri.
// Модель лежит в public/models/jane/简/简.model3.json (скопируй из assets/models/jane/)
import * as PIXI from "pixi.js";
import { Live2DModel } from "pixi-live2d-display";

let app: PIXI.Application | null = null;
let model: Live2DModel | null = null;

export async function initLive2D(container: HTMLElement, modelUrl: string): Promise<void> {
  // Cubism Core должен быть загружен через <script> в index.html (public/vendor/live2dcubismcore.min.js)
  app = new PIXI.Application({
    width: 400,
    height: 500,
    backgroundAlpha: 0,
    antialias: true,
  });
  container.appendChild(app.view as HTMLCanvasElement);
  model = await Live2DModel.from(modelUrl);
  model.scale.set(0.35);
  model.position.set(200, 480);
  model.anchor.set(0.5, 1.0);
  app.stage.addChild(model as unknown as PIXI.DisplayObject);
  try { (model as any).motion("Idle"); } catch {}
}

export function setMouth(v: number) {
  if (!model) return;
  try {
    (model as any).internalModel.coreModel.setParameterValueById("ParamMouthOpenY", Math.max(0, Math.min(1, v)));
  } catch {}
}

export function setExpression(name: string) {
  if (!model || !name) return;
  try { (model as any).expression(name); } catch {}
}

export function setLook(x: number, y: number) {
  if (!model) return;
  try {
    const core = (model as any).internalModel.coreModel;
    core.setParameterValueById("ParamAngleX", x * 30);
    core.setParameterValueById("ParamAngleY", y * 30);
    core.setParameterValueById("ParamEyeBallX", x);
    core.setParameterValueById("ParamEyeBallY", y);
  } catch {}
}
