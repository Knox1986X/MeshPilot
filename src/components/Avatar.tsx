import { useEffect, useRef } from "react";
import { initLive2D, setMouth } from "../lib/live2d";

export default function Avatar({ mouth = 0 }: { mouth?: number }) {
  const ref = useRef<HTMLDivElement>(null);
  const loaded = useRef(false);

  useEffect(() => {
    if (!ref.current || loaded.current) return;
    loaded.current = true;
    // Модель в public/models/jane/简/简.model3.json
    // Скопируй: cp -r assets/models/jane public/models/
    initLive2D(ref.current, "/models/jane/%E7%AE%80/%E7%AE%80.model3.json").catch((e) => {
      console.error("Live2D load failed:", e);
    });
  }, []);

  useEffect(() => {
    setMouth(mouth);
  }, [mouth]);

  return <div ref={ref} className="avatar-window" />;
}
