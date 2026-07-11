import { defineStore } from "pinia";
import { ref } from "vue";
import type { InstalledPlugin } from "@/types";
import {
  listPlugins as apiListPlugins,
  installPlugin as apiInstallPlugin,
  uninstallPlugin as apiUninstallPlugin,
  enablePlugin as apiEnablePlugin,
  configurePlugin as apiConfigurePlugin,
} from "@/api/plugin";

export const usePluginStore = defineStore("plugin", () => {
  const plugins = ref<InstalledPlugin[]>([]);
  const loading = ref(false);
  const initialized = ref(false);

  async function loadPlugins() {
    if (initialized.value && !loading.value) return;
    loading.value = true;
    try {
      plugins.value = await apiListPlugins();
      initialized.value = true;
    } catch (e) {
      console.warn("Failed to load plugins", e);
    } finally {
      loading.value = false;
    }
  }

  async function installPlugin(sourcePath: string) {
    loading.value = true;
    try {
      const installed = await apiInstallPlugin(sourcePath);
      const idx = plugins.value.findIndex((p) => p.id === installed.id);
      if (idx >= 0) {
        plugins.value[idx] = installed;
      } else {
        plugins.value.push(installed);
      }
      return installed;
    } finally {
      loading.value = false;
    }
  }

  async function uninstallPlugin(id: string) {
    loading.value = true;
    try {
      await apiUninstallPlugin(id);
      plugins.value = plugins.value.filter((p) => p.id !== id);
    } finally {
      loading.value = false;
    }
  }

  async function toggleEnabled(id: string) {
    const plugin = plugins.value.find((p) => p.id === id);
    if (!plugin) return;
    const next = !plugin.enabled;
    loading.value = true;
    try {
      const updated = await apiEnablePlugin(id, next);
      const idx = plugins.value.findIndex((p) => p.id === id);
      if (idx >= 0) plugins.value[idx] = updated;
      return updated;
    } finally {
      loading.value = false;
    }
  }

  async function updateConfig(id: string, config: Record<string, unknown>) {
    loading.value = true;
    try {
      const updated = await apiConfigurePlugin(id, config);
      const idx = plugins.value.findIndex((p) => p.id === id);
      if (idx >= 0) plugins.value[idx] = updated;
      return updated;
    } finally {
      loading.value = false;
    }
  }

  return {
    plugins,
    loading,
    initialized,
    loadPlugins,
    installPlugin,
    uninstallPlugin,
    toggleEnabled,
    updateConfig,
  };
});
