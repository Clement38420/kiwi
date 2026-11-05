<script setup lang="ts">
import { computed, onUnmounted, useTemplateRef, watch } from 'vue'
import { useJoystick } from '@/composables/useJoystick'
import { useRobotStore } from '@/stores/robot.ts'
import type { Twist } from '@/bindings/Twist.ts'

const MAX_SPEED = 1.0 // m/s
const SUSTAIN_INTERVAL_MS = 250

const robotStore = useRobotStore()
const joystickEl = useTemplateRef<HTMLElement>('joystick')

const { normalized, isDragging, style } = useJoystick(joystickEl, 60)

let sustainIntervalId: ReturnType<typeof setInterval> | null = null

const vx = computed(() => -normalized.value.y * MAX_SPEED)
const vy = computed(() => -normalized.value.x * MAX_SPEED)

function sendCurrentTwist() {
  const t: Twist = {
    vx: vx.value,
    vy: vy.value,
    w: 0,
  }
  robotStore.sendMessage({ Twist: t })
}

function stopSustain() {
  if (sustainIntervalId !== null) {
    clearInterval(sustainIntervalId)
    sustainIntervalId = null
  }
}

watch(normalized, () => {
  sendCurrentTwist()
})

watch(isDragging, (dragging) => {
  stopSustain()
  if (dragging) {
    sustainIntervalId = setInterval(sendCurrentTwist, SUSTAIN_INTERVAL_MS)
  } else {
    sendCurrentTwist()
  }
})

onUnmounted(stopSustain)
</script>

<template>
  <div class="group w-auto h-auto flex flex-col">
    <div class="group-title">Control</div>
    <div class="relative flex-1 flex justify-center items-center">
      <div class="relative w-0 h-0">
        <svg
          v-for="i in [0, 1, 2, 3]"
          :key="i"
          :style="{
            top: `${i % 2 === 0 ? (i - 1) * 60 : 0}px`,
            left: `${i % 2 === 1 ? (2 - i) * 60 : 0}px`,
            rotate: `${i * 90}deg`,
          }"
          class="absolute -translate-1/2 text-graphite-200 dark:text-graphite-700"
          height="20px"
          width="20px"
          viewBox="0 0 50 50"
        >
          <polygon
            points="5,35 45,35 25,10"
            fill="currentColor"
            stroke="currentColor"
            stroke-width="4"
            stroke-linejoin="round"
          ></polygon>
        </svg>

        <div
          class="absolute top-0 left-0 -translate-1/2 rounded-full bg-graphite-200 dark:bg-graphite-800 size-20 touch-none cursor-grab active:cursor-grabbing select-none"
        >
          <div
            ref="joystick"
            :style="style"
            class="absolute inset-3 rounded-full bg-graphite-100 dark:bg-graphite-700 shadow active:brightness-104 dark:active:brightness-130 transition-[filter]"
          >
            <div
              class="absolute inset-3 rounded-full bg-graphite-100 dark:bg-graphite-700 inset-shadow-sm inset-shadow-graphite-200 dark:inset-shadow-graphite-800 rotate-12 pointer-events-none"
            ></div>
          </div>
        </div>
      </div>
    </div>
    <div class="flex justify-center items-center w-full gap-4 font-mono">
      <div class="rounded-field bg-surface-2 py-2 px-4 flex-1 flex justify-between border">
        <span class="text-kiwi-text">Vx</span>
        <span>{{ vx.toFixed(2) }}</span>
      </div>
      <div class="rounded-field bg-surface-2 py-2 px-4 flex-1 flex justify-between border">
        <span class="text-kiwi-text">Vy</span>
        <span>{{ vy.toFixed(2) }}</span>
      </div>
    </div>
  </div>
</template>
