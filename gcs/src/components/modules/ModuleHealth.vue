<script setup lang="ts">
import { useRobotStore } from '@/stores/robot.ts'
import { computed } from 'vue'
import BaseProgressBar from '@/components/base/BaseProgressBar.vue'

const kiwiStore = useRobotStore()

interface Vital {
  label: string
  value: string
  dot?: 'living-dot' | 'bg-warn' | 'bg-danger' | 'bg-ok' | 'bg-info'
  progressBar?: {
    color: string
    progress: number
  }
}

const connectionVital = computed<Vital>(() => {
  const label = 'Robot connection'
  switch (kiwiStore.connectionStatus) {
    case WebSocket.CONNECTING:
      return { label, value: 'Connecting...', dot: 'bg-warn' }
    case WebSocket.OPEN:
      return { label, value: 'Connected', dot: 'living-dot' }
    case WebSocket.CLOSING:
      return { label, value: 'Disconnecting...', dot: 'bg-warn' }
    case WebSocket.CLOSED:
    default:
      return { label, value: 'Disconnected', dot: 'bg-danger' }
  }
})

const BATTERY_MIN_VOLTAGE = 17.4
const BATTERY_MAX_VOLTAGE = 21.5

const vitals = computed<Vital[]>(() => {
  const batteryProgress = Math.max(
    0,
    Math.min(
      1,
      (kiwiStore.batteryVoltage - BATTERY_MIN_VOLTAGE) /
        (BATTERY_MAX_VOLTAGE - BATTERY_MIN_VOLTAGE),
    ),
  )

  return [
    connectionVital.value,
    {
      label: 'Battery voltage',
      value: `${kiwiStore.batteryVoltage.toFixed(2)} V`,
      progressBar: {
        color: 'bg-kiwi',
        progress: batteryProgress,
      },
    },
  ]
})
</script>

<template>
  <div class="group w-full md:w-1/2 xl:w-1/3 text-sm">
    <p class="group-title">Health</p>
    <div class="card w-full">
      <div class="tel-list">
        <div v-for="vital in vitals" :key="vital.label" class="tel-row">
          <div class="tel-header">
            <span class="tel-label">{{ vital.label }}</span>
            <span class="tel-value">
              <span v-if="vital.dot" :class="['dot', vital.dot]" />
              <span>{{ vital.value }}</span>
            </span>
          </div>
          <BaseProgressBar
            v-if="vital.progressBar"
            :color="vital.progressBar.color"
            :progress="vital.progressBar.progress"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped></style>
