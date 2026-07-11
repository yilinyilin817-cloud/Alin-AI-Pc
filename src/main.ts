import { createApp } from "vue";
import { createPinia } from "pinia";
import ElementPlus from "element-plus";
import zhCn from "element-plus/es/locale/lang/zh-cn";
import App from "./App.vue";
import router from "./router";
import "element-plus/dist/index.css";
import "@/assets/styles/global.scss";
import "@/styles/design-system.css";

const app = createApp(App);
const pinia = createPinia();
app.use(pinia);
app.use(router);
app.use(ElementPlus, { locale: zhCn });

// 应用启动时初始化关键 store
(async () => {
  // 先加载主题（从 localStorage 恢复），避免启动闪烁
  const savedTheme = localStorage.getItem("theme");
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const shouldLight = savedTheme === "light" || (savedTheme === "system" && !prefersDark);
  if (shouldLight) {
    document.documentElement.classList.add("light");
    document.documentElement.classList.remove("dark");
  } else {
    document.documentElement.classList.remove("light");
    document.documentElement.classList.add("dark");
  }

  const { usePersonaStore } = await import("@/stores/persona");
  const personaStore = usePersonaStore(pinia);
  personaStore.loadPersonas().catch((e) => console.warn("loadPersonas:", e));

  const { useSettingsStore } = await import("@/stores/settings");
  const settingsStore = useSettingsStore(pinia);
  await settingsStore.load().catch((e) => console.warn("loadSettings:", e));

  // 恢复上次访问的路由（仅恢复基础路径，动态 id 由对应页面自行处理）
  const lastRoute = settingsStore.lastRoute;
  if (lastRoute && lastRoute !== router.currentRoute.value.path) {
    router.replace(lastRoute).catch((e) => console.warn("restore lastRoute:", e));
  }
})();

app.mount("#app");
