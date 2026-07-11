import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useSettingsStore, type FontSize, type MessageDensity } from "@/stores/settings";

const mediaQuery =
  typeof window !== "undefined"
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;

export function useTheme() {
  const settings = useSettingsStore();
  const systemIsDark = ref(mediaQuery?.matches ?? false);

  function onSystemChange(event: MediaQueryListEvent | MediaQueryList) {
    systemIsDark.value = "matches" in event ? event.matches : false;
  }

  const effectiveTheme = computed<"dark" | "light">(() =>
    settings.theme === "system"
      ? systemIsDark.value
        ? "dark"
        : "light"
      : settings.theme,
  );

  function applyThemeClass() {
    const root = document.documentElement;
    if (effectiveTheme.value === "light") {
      root.classList.add("light");
      root.classList.remove("dark");
    } else {
      root.classList.remove("light");
      root.classList.add("dark");
    }
  }

  function applyFontSizeClass() {
    const root = document.documentElement;
    const sizes: FontSize[] = ["small", "default", "large"];
    sizes.forEach((s) => root.classList.remove(`text-size-${s}`));
    root.classList.add(`text-size-${settings.fontSize}`);
  }

  function applyDensityClass() {
    const root = document.documentElement;
    const densities: MessageDensity[] = ["compact", "default", "cozy"];
    densities.forEach((d) => root.classList.remove(`density-${d}`));
    root.classList.add(`density-${settings.messageDensity}`);
  }

  function toggleTheme() {
    const next = effectiveTheme.value === "dark" ? "light" : "dark";
    settings.setTheme(next);
  }

  watch(() => settings.theme, applyThemeClass);
  watch(systemIsDark, applyThemeClass);
  watch(() => settings.fontSize, applyFontSizeClass);
  watch(() => settings.messageDensity, applyDensityClass);

  onMounted(() => {
    mediaQuery?.addEventListener("change", onSystemChange);
    applyThemeClass();
    applyFontSizeClass();
    applyDensityClass();
  });

  onUnmounted(() => {
    mediaQuery?.removeEventListener("change", onSystemChange);
  });

  return {
    effectiveTheme,
    systemIsDark,
    toggleTheme,
    applyThemeClass,
    applyFontSizeClass,
    applyDensityClass,
  };
}
