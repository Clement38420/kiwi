<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  options: string[]
  disabled?: boolean
}>()

const selected = defineModel<number>({ default: 0 })

const activeIndex = computed(() => {
  if (props.options.length === 0) return 0
  return Math.min(Math.max(selected.value, 0), props.options.length - 1)
})

const indicatorStyle = computed(() => {
  const count = Math.max(props.options.length, 1)
  return {
    width: `calc((100% - 0.25rem) / ${String(count)})`,
    transform: `translateX(${String(activeIndex.value * 100)}%)`,
  }
})
</script>

<template>
  <div
    role="tablist"
    class="relative grid grid-flow-col auto-cols-fr items-center p-0.5 -my-0.75 rounded-control border border-line bg-surface-2 dark:bg-background shadow-[inset_0_1px_2px_rgb(15_23_42/0.06)] dark:shadow-[inset_0_1px_2px_rgb(0_0_0/0.45)] select-none"
    :class="disabled ? 'opacity-50 pointer-events-none' : ''"
  >
    <span
      v-if="options.length > 0"
      aria-hidden="true"
      class="pointer-events-none absolute top-0.5 bottom-0.5 left-0.5 rounded-[calc(var(--radius-control)-2px)] bg-surface dark:bg-surface-2 border border-line/80 dark:border-graphite-700 shadow-panel dark:shadow-[inset_0_1px_0_rgb(255_255_255/0.07),0_1px_3px_rgb(0_0_0/0.45)] transition-transform duration-200 ease-kiwi"
      :style="indicatorStyle"
    />
    <button
      v-for="(option, idx) in options"
      :key="idx"
      type="button"
      role="tab"
      :aria-selected="activeIndex === idx"
      :disabled="disabled"
      class="relative z-10 flex items-center justify-center rounded-[calc(var(--radius-control)-2px)] px-3.5 py-1.5 text-sm font-semibold transition-colors duration-150 ease-kiwi active:enabled:translate-y-px"
      :class="activeIndex === idx ? 'text-ink' : 'text-ink-muted hover:text-ink'"
      @click="selected = idx"
    >
      {{ option }}
    </button>
  </div>
</template>
