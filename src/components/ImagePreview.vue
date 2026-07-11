<script setup lang="ts">
defineProps<{
  src: string;
  alt?: string;
}>();
</script>

<template>
  <el-image
    :src="src"
    :alt="alt ?? 'preview'"
    fit="cover"
    class="image-preview"
    :preview-src-list="[src]"
    preview-teleported
  />
</template>

<style scoped lang="scss">
.image-preview {
  border-radius: var(--radius-md);
  max-width: 100%;
  border: 1px solid var(--glass-border);
  box-shadow: var(--glass-shadow);
  transition: all var(--transition);

  &:hover {
    border-color: var(--glass-border-hover);
    transform: scale(1.01);
  }
}
</style>
