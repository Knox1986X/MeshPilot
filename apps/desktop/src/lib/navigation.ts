import { Activity, Globe, Settings, ShieldCheck, Trash2 } from "@lucide/svelte";

export const views = [
  { id: "dashboard", name: "Dashboard", icon: Activity, sub: "Локальные измерения · CPU и RAM" },
  { id: "cleanup", name: "CleanUp", icon: Trash2, sub: "Категории очистки и предварительная оценка" },
  { id: "network", name: "Mirrors & DNS", icon: Globe, sub: "Сетевой профиль, DNS и зеркала" },
  { id: "permissions", name: "Permissions", icon: ShieldCheck, sub: "Права и границы системного доступа" },
  { id: "settings", name: "Settings & Tray", icon: Settings, sub: "Настройки окна и фоновой работы" },
] as const;
export type ViewId = (typeof views)[number]["id"];
export function resolveView(hash: string): ViewId {
  return views.find((v) => `#/${v.id}` === hash)?.id ?? "dashboard";
}
