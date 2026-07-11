<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { getRelationship, type RelationshipInfo } from '@/api/companion'
import { usePersonaStore } from '@/stores/persona'

const personaStore = usePersonaStore()
const info = ref<RelationshipInfo | null>(null)
const showMilestone = ref(false)
const newMilestone = ref<{ title: string; description: string; icon: string } | null>(null)

async function load() {
  const pid = personaStore.currentPersonaId
  if (!pid) return
  try {
    info.value = await getRelationship(pid)
  } catch {
    // not running in tauri
  }
}

const intimacyPercent = computed(() => Math.round(info.value?.state.intimacy ?? 0))
const daysKnown = computed(() => info.value?.state.daysKnown ?? 0)
const styleLabel = computed(() => {
  const s = info.value?.state.responseStyle
  switch (s) {
    case 'Passionate': return '热恋中'
    case 'Intimate': return '亲密'
    case 'Friendly': return '朋友'
    default: return '初识'
  }
})
const heartColor = computed(() => {
  const v = intimacyPercent.value
  if (v >= 80) return '#ff4d6d'
  if (v >= 50) return '#ff758f'
  if (v >= 20) return '#ffb3c1'
  return '#c9ccd3'
})

watch(() => personaStore.currentPersonaId, load)
onMounted(load)

defineExpose({ load, showMilestonePopup: (m: typeof newMilestone.value) => {
  newMilestone.value = m
  showMilestone.value = true
  setTimeout(() => { showMilestone.value = false }, 4000)
}})
</script>

<template>
  <div v-if="info" class="companion-bar">
    <div class="companion-info">
      <span class="heart" :style="{ color: heartColor }">❤</span>
      <span class="days" v-if="daysKnown > 0">第{{ daysKnown }}天</span>
      <span class="style-badge" :class="styleLabel">{{ styleLabel }}</span>
    </div>
    <div class="intimacy-bar">
      <div 
        class="intimacy-fill"
        :style="{ width: intimacyPercent + '%', background: `linear-gradient(90deg, #ffb3c1, ${heartColor})` }"
      />
    </div>
    <div class="intimacy-text">{{ intimacyPercent }}/100</div>

    <Transition name="milestone-pop">
      <div v-if="showMilestone && newMilestone" class="milestone-popup">
        <span class="milestone-icon">{{ newMilestone.icon }}</span>
        <div class="milestone-text">
          <div class="milestone-title">{{ newMilestone.title }}</div>
          <div class="milestone-desc">{{ newMilestone.description }}</div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.companion-bar {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 16px;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  font-size: 12px;
  color: var(--color-text-muted);
  flex-shrink: 0;
}

.companion-info {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 90px;
}

.heart {
  font-size: 14px;
  filter: drop-shadow(0 0 4px rgba(255, 77, 109, 0.4));
}

.days {
  color: var(--color-text-secondary);
}

.style-badge {
  padding: 1px 8px;
  border-radius: 10px;
  font-size: 11px;
  background: rgba(255, 255, 255, 0.06);
}

.style-badge.热恋中 {
  background: rgba(255, 77, 109, 0.2);
  color: #ff758f;
}

.style-badge.亲密 {
  background: rgba(255, 117, 143, 0.15);
  color: #ffb3c1;
}

.intimacy-bar {
  flex: 1;
  height: 4px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 2px;
  overflow: hidden;
}

.intimacy-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.6s ease;
}

.intimacy-text {
  font-variant-numeric: tabular-nums;
  min-width: 50px;
  text-align: right;
  color: var(--color-text-secondary);
}

.milestone-popup {
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-top: 8px;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 20px;
  background: linear-gradient(135deg, rgba(255, 77, 109, 0.15), rgba(255, 179, 193, 0.1));
  border: 1px solid rgba(255, 117, 143, 0.3);
  border-radius: 12px;
  backdrop-filter: blur(12px);
  z-index: 100;
  box-shadow: 0 8px 32px rgba(255, 77, 109, 0.2);
}

.milestone-icon {
  font-size: 32px;
  animation: bounce 0.6s ease;
}

.milestone-title {
  font-size: 14px;
  font-weight: 600;
  color: #ff758f;
}

.milestone-desc {
  font-size: 12px;
  color: var(--color-text-secondary);
  margin-top: 2px;
}

@keyframes bounce {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.3); }
}

.milestone-pop-enter-active {
  animation: pop-in 0.4s ease;
}

.milestone-pop-leave-active {
  animation: pop-out 0.3s ease;
}

@keyframes pop-in {
  0% { opacity: 0; transform: translateX(-50%) translateY(-10px) scale(0.8); }
  100% { opacity: 1; transform: translateX(-50%) translateY(0) scale(1); }
}

@keyframes pop-out {
  0% { opacity: 1; transform: translateX(-50%) translateY(0) scale(1); }
  100% { opacity: 0; transform: translateX(-50%) translateY(-10px) scale(0.8); }
}
</style>
