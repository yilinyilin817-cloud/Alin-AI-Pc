<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from "vue";

type ThemeMode = "dark" | "light" | "auto";
type Mood = "calm" | "energetic" | "mystic" | "warm";

const props = withDefaults(
  defineProps<{
    seed?: number | string;
    theme?: ThemeMode;
    mood?: Mood;
    intensity?: number;
    animate?: boolean;
  }>(),
  {
    seed: () => Date.now(),
    theme: "auto",
    mood: "mystic",
    intensity: 0.6,
    animate: true,
  },
);

const containerRef = ref<HTMLElement | null>(null);
let p5Instance: any = null;
let themeObserver: MutationObserver | null = null;
let visibilityCleanup: (() => void) | null = null;

interface WindowWithP5 extends Window {
  p5?: any;
}

/* ═══════════════════════════════════════════════════════════
   确定性随机：seed -> 可复现序列
   ═══════════════════════════════════════════════════════════ */

function cyrb53(str: string): number {
  let h1 = 0xdeadbeef;
  let h2 = 0x41c6ce57;
  for (let i = 0; i < str.length; i++) {
    const ch = str.charCodeAt(i);
    h1 = Math.imul(h1 ^ ch, 2654435761);
    h2 = Math.imul(h2 ^ ch, 1597334677);
  }
  h1 =
    Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^
    Math.imul(h2 ^ (h2 >>> 13), 3266489909);
  h2 =
    Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^
    Math.imul(h1 ^ (h1 >>> 13), 3266489909);
  return 4294967296 * (2097151 & h2) + (h1 >>> 0);
}

function seedNumber(seed: number | string): number {
  return typeof seed === "number" ? seed : cyrb53(seed);
}

function mulberry32(seed: number) {
  let t = seed >>> 0;
  return function () {
    t += 0x6d2b79f5;
    let r = Math.imul(t ^ (t >>> 15), t | 1);
    r ^= r + Math.imul(r ^ (r >>> 7), r | 61);
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
  };
}

/* ═══════════════════════════════════════════════════════════
   自定义三维值噪声（Perlin 梯度噪声近似）
   ═══════════════════════════════════════════════════════════ */

class ValueNoise {
  private perm: number[] = [];

  constructor(rng: () => number) {
    const p = Array.from({ length: 256 }, (_, i) => i);
    for (let i = 255; i > 0; i--) {
      const j = Math.floor(rng() * (i + 1));
      [p[i], p[j]] = [p[j], p[i]];
    }
    this.perm = [...p, ...p];
  }

  private fade(t: number) {
    return t * t * t * (t * (t * 6 - 15) + 10);
  }

  private lerp(a: number, b: number, t: number) {
    return a + (b - a) * t;
  }

  private grad(hash: number, x: number, y: number, z: number) {
    const h = hash & 15;
    const u = h < 8 ? x : y;
    const v = h < 4 ? y : h === 12 || h === 14 ? x : z;
    return ((h & 1) === 0 ? u : -u) + ((h & 2) === 0 ? v : -v);
  }

  noise3D(x: number, y: number, z: number): number {
    const X = Math.floor(x) & 255;
    const Y = Math.floor(y) & 255;
    const Z = Math.floor(z) & 255;
    x -= Math.floor(x);
    y -= Math.floor(y);
    z -= Math.floor(z);
    const u = this.fade(x);
    const v = this.fade(y);
    const w = this.fade(z);
    const p = this.perm;
    const A = p[X] + Y;
    const AA = p[A] + Z;
    const AB = p[A + 1] + Z;
    const B = p[X + 1] + Y;
    const BA = p[B] + Z;
    const BB = p[B + 1] + Z;

    return this.lerp(
      this.lerp(
        this.lerp(this.grad(p[AA], x, y, z), this.grad(p[BA], x - 1, y, z), u),
        this.lerp(this.grad(p[AB], x, y - 1, z), this.grad(p[BB], x - 1, y - 1, z), u),
        v,
      ),
      this.lerp(
        this.lerp(this.grad(p[AA + 1], x, y, z - 1), this.grad(p[BA + 1], x - 1, y, z - 1), u),
        this.lerp(this.grad(p[AB + 1], x, y - 1, z - 1), this.grad(p[BB + 1], x - 1, y - 1, z - 1), u),
        v,
      ),
      w,
    );
  }
}

/* ═══════════════════════════════════════════════════════════
   CDN 加载 p5.js
   ═══════════════════════════════════════════════════════════ */

let p5LoadPromise: Promise<void> | null = null;

function loadP5Script(): Promise<void> {
  const w = window as unknown as WindowWithP5;
  if (w.p5) return Promise.resolve();
  if (p5LoadPromise) return p5LoadPromise;
  p5LoadPromise = new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = "https://cdnjs.cloudflare.com/ajax/libs/p5.js/1.7.0/p5.min.js";
    script.async = true;
    script.crossOrigin = "anonymous";
    script.onload = () => resolve();
    script.onerror = () => reject(new Error("Failed to load p5.js from CDN"));
    document.head.appendChild(script);
  });
  return p5LoadPromise;
}

/* ═══════════════════════════════════════════════════════════
   主题 / 气质 配置
   ═══════════════════════════════════════════════════════════ */

function isLightTheme(): boolean {
  if (props.theme === "dark") return false;
  if (props.theme === "light") return true;
  return document.documentElement.classList.contains("light");
}

interface MoodConfig {
  speed: number;
  noiseScale: number;
  noiseOctaves: number;
  trail: number;
  colorShift: number;
  saturation: number;
  lightness: number;
  glow: number;
}

function moodConfig(mood: Mood): MoodConfig {
  switch (mood) {
    case "calm":
      return {
        speed: 0.35,
        noiseScale: 0.0025,
        noiseOctaves: 2,
        trail: 22,
        colorShift: 0,
        saturation: 0.55,
        lightness: 0.58,
        glow: 0.25,
      };
    case "energetic":
      return {
        speed: 1.4,
        noiseScale: 0.0065,
        noiseOctaves: 3,
        trail: 9,
        colorShift: 0.08,
        saturation: 0.9,
        lightness: 0.62,
        glow: 0.65,
      };
    case "warm":
      return {
        speed: 0.65,
        noiseScale: 0.0035,
        noiseOctaves: 2,
        trail: 16,
        colorShift: -0.08,
        saturation: 0.8,
        lightness: 0.6,
        glow: 0.4,
      };
    case "mystic":
    default:
      return {
        speed: 0.55,
        noiseScale: 0.004,
        noiseOctaves: 3,
        trail: 18,
        colorShift: 0.12,
        saturation: 0.72,
        lightness: 0.54,
        glow: 0.55,
      };
  }
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => {
    const k = (n + h * 12) % 12;
    return l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
  };
  return [f(0), f(8), f(4)].map((v) => Math.round(v * 255)) as [number, number, number];
}

function themeColors(light: boolean) {
  return light
    ? {
        primary: [124, 107, 240] as [number, number, number],
        accent: [245, 168, 200] as [number, number, number],
        bg: [245, 246, 250] as [number, number, number],
      }
    : {
        primary: [124, 107, 240] as [number, number, number],
        accent: [245, 168, 200] as [number, number, number],
        bg: [26, 27, 38] as [number, number, number],
      };
}

/* ═══════════════════════════════════════════════════════════
   p5 sketch 构造器
   ═══════════════════════════════════════════════════════════ */

function createSketch(seed: number, light: boolean, mood: Mood, intensity: number, animate: boolean) {
  return (p: any) => {
    const rng = mulberry32(seed);
    const noise = new ValueNoise(rng);
    const cfg = moodConfig(mood);
    const colors = themeColors(light);
    const particleCount = Math.floor(200 + Math.max(0, Math.min(1, intensity)) * 200);
    const particles: {
      x: number;
      y: number;
      vx: number;
      vy: number;
      age: number;
      life: number;
      size: number;
      hueOffset: number;
    }[] = [];
    let time = 0;

    function spawnParticle() {
      const life = 200 + rng() * 360;
      return {
        x: rng() * p.width,
        y: rng() * p.height,
        vx: 0,
        vy: 0,
        age: rng() * life,
        life,
        size: 1.1 + rng() * 2.4,
        hueOffset: rng(),
      };
    }

    p.setup = () => {
      const w = containerRef.value?.clientWidth || 320;
      const h = containerRef.value?.clientHeight || 240;
      p.pixelDensity(1);
      p.frameRate(30);
      const canvas = p.createCanvas(w, h);
      canvas.style("display", "block");
      for (let i = 0; i < particleCount; i++) particles.push(spawnParticle());
      p.background(colors.bg[0], colors.bg[1], colors.bg[2]);
      if (!animate) p.noLoop();
    };

    p.draw = () => {
      // 拖尾：以低透明度背景覆盖，形成有机残影
      p.noStroke();
      p.fill(colors.bg[0], colors.bg[1], colors.bg[2], cfg.trail);
      p.rect(0, 0, p.width, p.height);

      const t = time * cfg.speed * 0.002;
      const ns = cfg.noiseScale;
      const lightBoost = light ? 1.08 : 1.0;

      for (let i = 0; i < particles.length; i++) {
        const pt = particles[i];

        // 多层噪声叠加的流场角度
        let angle = 0;
        let amp = 1;
        let freq = 1;
        let norm = 0;
        for (let o = 0; o < cfg.noiseOctaves; o++) {
          angle += noise.noise3D(pt.x * ns * freq, pt.y * ns * freq, t * freq) * Math.PI * 2 * amp;
          norm += amp;
          amp *= 0.5;
          freq *= 2;
        }
        angle /= norm;
        angle += cfg.colorShift * Math.PI * 2;

        const force = 0.12 * cfg.speed;
        pt.vx += Math.cos(angle) * force;
        pt.vy += Math.sin(angle) * force;
        pt.vx *= 0.93;
        pt.vy *= 0.93;

        pt.x += pt.vx;
        pt.y += pt.vy;
        pt.age++;

        const outOfBounds = pt.x < -12 || pt.x > p.width + 12 || pt.y < -12 || pt.y > p.height + 12;
        if (pt.age > pt.life || outOfBounds) {
          particles[i] = spawnParticle();
          continue;
        }

        const lifeRatio = pt.age / pt.life;
        const alphaBase = Math.sin(lifeRatio * Math.PI) * (0.35 + intensity * 0.35);
        const alpha = alphaBase * (light ? 200 : 255);

        const hueBlend = pt.hueOffset + cfg.colorShift;
        const primary = hslToRgb((260 + hueBlend * 40) / 360, cfg.saturation, cfg.lightness * lightBoost);
        const accent = hslToRgb((330 + hueBlend * 30) / 360, cfg.saturation, cfg.lightness * lightBoost);
        const c = [
          primary[0] * (1 - lifeRatio) + accent[0] * lifeRatio,
          primary[1] * (1 - lifeRatio) + accent[1] * lifeRatio,
          primary[2] * (1 - lifeRatio) + accent[2] * lifeRatio,
        ];

        p.fill(c[0], c[1], c[2], alpha);
        p.ellipse(pt.x, pt.y, pt.size, pt.size);
      }

      // mystic 气质增加微弱暗角，强化空间纵深感
      if (mood === "mystic" && cfg.glow > 0) {
        const vignetteAlpha = light ? 2 : 4;
        p.fill(colors.bg[0], colors.bg[1], colors.bg[2], vignetteAlpha);
        p.rect(0, 0, p.width, p.height);
      }

      time++;
    };

    p.windowResized = () => {
      const w = containerRef.value?.clientWidth || 320;
      const h = containerRef.value?.clientHeight || 240;
      p.resizeCanvas(w, h);
      p.background(colors.bg[0], colors.bg[1], colors.bg[2]);
    };
  };
}

async function mountSketch() {
  if (!containerRef.value) return;
  try {
    await loadP5Script();
  } catch (e) {
    console.warn("GenerativeArtBackground: p5.js load failed", e);
    return;
  }
  if (!containerRef.value) return; // 等待期间组件可能已卸载
  if (p5Instance) {
    p5Instance.remove();
    p5Instance = null;
  }
  const seed = seedNumber(props.seed);
  const light = isLightTheme();
  const P5 = (window as unknown as WindowWithP5).p5;
  p5Instance = new P5(createSketch(seed, light, props.mood, props.intensity, props.animate), containerRef.value);

  // 窗口/标签页不可见时暂停动画，降低 GPU 占用
  visibilityCleanup?.();
  const _p5 = p5Instance;
  const handleVisibility = () => {
    if (!_p5) return;
    if (document.hidden) {
      _p5.noLoop();
    } else if (props.animate) {
      _p5.loop();
    }
  };
  document.addEventListener("visibilitychange", handleVisibility);
  visibilityCleanup = () => document.removeEventListener("visibilitychange", handleVisibility);
}

watch(
  () => [props.seed, props.theme, props.mood, props.intensity, props.animate],
  () => mountSketch(),
);

onMounted(() => {
  mountSketch();

  // auto 模式下监听 html class 变化
  themeObserver = new MutationObserver(() => {
    if (props.theme === "auto") mountSketch();
  });
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["class"],
  });
});

onUnmounted(() => {
  visibilityCleanup?.();
  visibilityCleanup = null;
  themeObserver?.disconnect();
  themeObserver = null;
  if (p5Instance) {
    p5Instance.remove();
    p5Instance = null;
  }
});
</script>

<template>
  <div ref="containerRef" class="generative-art-background" />
</template>

<style scoped lang="scss">
.generative-art-background {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  pointer-events: none;

  :deep(canvas) {
    display: block;
    width: 100% !important;
    height: 100% !important;
  }
}
</style>
