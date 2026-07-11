<script setup lang="ts">
import { computed } from "vue";
import type { Component } from "vue";

type ButtonVariant = "primary" | "default" | "ghost" | "danger";
type ButtonSize = "small" | "default" | "large";
type NativeType = "button" | "submit" | "reset";

interface Props {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: Component;
  disabled?: boolean;
  loading?: boolean;
  circle?: boolean;
  round?: boolean;
  nativeType?: NativeType;
}

const props = withDefaults(defineProps<Props>(), {
  variant: "default",
  size: "default",
  nativeType: "button",
});

const elType = computed(() => {
  if (props.variant === "primary") return "primary";
  if (props.variant === "danger") return "danger";
  return "default";
});

const classes = computed(() => [
  "base-button",
  `base-button--${props.variant}`,
]);
</script>

<template>
  <el-button
    :type="elType"
    :size="size"
    :icon="icon"
    :disabled="disabled"
    :loading="loading"
    :circle="circle"
    :round="round"
    :native-type="nativeType"
    :class="classes"
  >
    <slot />
  </el-button>
</template>

<style scoped lang="scss">
.base-button {
  font-weight: 500;
  transition: all var(--transition-base);

  &:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
}

.base-button--default {
  background: var(--color-bg-surface);
  border-color: var(--color-border);
  color: var(--color-text-primary);

  &:hover,
  &:focus {
    background: var(--color-bg-hover);
    border-color: var(--color-primary-light);
    color: var(--color-primary-light);
  }
}

.base-button--ghost {
  background: transparent !important;
  border-color: var(--glass-border) !important;
  color: var(--color-text-primary) !important;

  &:hover,
  &:focus {
    background: var(--glass-hover-bg) !important;
    border-color: var(--glass-border-hover) !important;
    color: var(--color-primary) !important;
  }

  &:active {
    background: var(--glass-active-bg) !important;
  }
}
</style>
